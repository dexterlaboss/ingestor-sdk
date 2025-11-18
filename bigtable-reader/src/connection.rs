use {
    crate::{
        error::Error,
        types::*,
        bigtable::BigTable,
    },
    solana_bigtable_shared::{
        access_token::{AccessToken, Scope},
        root_ca_certificate, CredentialType,
    },
    backoff::{future::retry, ExponentialBackoff},
    log::*,
    std::{
        str::FromStr,
        time::{
            Duration,
        },
    },
    tonic::{
        transport::ClientTlsConfig, Request, Status
    },
};

use solana_bigtable_shared::google::bigtable::v2::*;

pub type InterceptedRequestResult = std::result::Result<Request<()>, Status>;

#[derive(Clone)]
pub struct BigTableConnection {
    access_token: Option<AccessToken>,
    channel: tonic::transport::Channel,
    table_prefix: String,
    app_profile_id: String,
    timeout: Option<Duration>,
}

impl BigTableConnection {
    /// Establish a connection to the BigTable instance named `instance_name`.  If read-only access
    /// is required, the `read_only` flag should be used to reduce the requested OAuth2 scope.
    ///
    /// The GOOGLE_APPLICATION_CREDENTIALS environment variable will be used to determine the
    /// program name that contains the BigTable instance in addition to access credentials.
    ///
    /// The BIGTABLE_EMULATOR_HOST environment variable is also respected.
    ///
    /// The BIGTABLE_PROXY environment variable is used to configure the gRPC connection through a
    /// forward proxy (see HTTP_PROXY).
    ///
    pub async fn new(
        instance_name: &str,
        app_profile_id: &str,
        read_only: bool,
        timeout: Option<Duration>,
        credential_type: CredentialType,
    ) -> Result<Self> {
        match std::env::var("BIGTABLE_EMULATOR_HOST") {
            Ok(endpoint) => {
                info!("Connecting to bigtable emulator at {}", endpoint);
                Self::new_for_emulator(instance_name, app_profile_id, &endpoint, timeout)
            }

            Err(_) => {
                let access_token = AccessToken::new(
                    if read_only {
                        Scope::BigTableDataReadOnly
                    } else {
                        Scope::BigTableData
                    },
                    credential_type,
                )
                    .await
                    .map_err(Error::AccessToken)?;

                let table_prefix = format!(
                    "projects/{}/instances/{}/tables/",
                    access_token.project(),
                    instance_name
                );

                let endpoint = {
                    let endpoint =
                        tonic::transport::Channel::from_static("https://bigtable.googleapis.com")
                            .tls_config(
                                ClientTlsConfig::new()
                                    .ca_certificate(
                                        root_ca_certificate::load().map_err(Error::Certificate)?,
                                    )
                                    .domain_name("bigtable.googleapis.com"),
                            )?;

                    if let Some(timeout) = timeout {
                        endpoint.timeout(timeout)
                    } else {
                        endpoint
                    }
                };

                let mut http = hyper::client::HttpConnector::new();
                http.enforce_http(false);
                http.set_nodelay(true);
                let channel = match std::env::var("BIGTABLE_PROXY") {
                    Ok(proxy_uri) => {
                        let proxy = hyper_proxy::Proxy::new(
                            hyper_proxy::Intercept::All,
                            proxy_uri
                                .parse::<http::Uri>()
                                .map_err(|err| Error::InvalidUri(proxy_uri, err.to_string()))?,
                        );
                        let mut proxy_connector =
                            hyper_proxy::ProxyConnector::from_proxy(http, proxy)?;
                        // tonic handles TLS as a separate layer
                        proxy_connector.set_tls(None);
                        endpoint.connect_with_connector_lazy(proxy_connector)
                    }
                    _ => endpoint.connect_with_connector_lazy(http),
                };

                Ok(Self {
                    access_token: Some(access_token),
                    channel,
                    table_prefix,
                    app_profile_id: app_profile_id.to_string(),
                    timeout,
                })
            }
        }
    }

    pub fn new_for_emulator(
        instance_name: &str,
        app_profile_id: &str,
        endpoint: &str,
        timeout: Option<Duration>,
    ) -> Result<Self> {
        Ok(Self {
            access_token: None,
            channel: tonic::transport::Channel::from_shared(format!("http://{endpoint}"))
                .map_err(|err| Error::InvalidUri(String::from(endpoint), err.to_string()))?
                .connect_lazy(),
            table_prefix: format!("projects/emulator/instances/{instance_name}/tables/"),
            app_profile_id: app_profile_id.to_string(),
            timeout,
        })
    }

    /// Create a new BigTable client.
    ///
    /// Clients require `&mut self`, due to `Tonic::transport::Channel` limitations, however
    /// creating new clients is cheap and thus can be used as a work around for ease of use.
    pub fn client(&self) -> BigTable<impl FnMut(Request<()>) -> InterceptedRequestResult> {
        let access_token = self.access_token.clone();
        let client = bigtable_client::BigtableClient::with_interceptor(
            self.channel.clone(),
            move |mut req: Request<()>| {
                if let Some(access_token) = &access_token {
                    match FromStr::from_str(&access_token.get()) {
                        Ok(authorization_header) => {
                            req.metadata_mut()
                                .insert("authorization", authorization_header);
                        }
                        Err(err) => {
                            warn!("Failed to set authorization header: {}", err);
                        }
                    }
                }
                Ok(req)
            },
        );
        BigTable {
            access_token: self.access_token.clone(),
            client,
            table_prefix: self.table_prefix.clone(),
            app_profile_id: self.app_profile_id.clone(),
            timeout: self.timeout,
        }
    }

    pub async fn get_bincode_cells_with_retry<T>(
        &self,
        table: &str,
        row_keys: &[RowKey],
    ) -> Result<Vec<(RowKey, Result<T>)>>
    where
        T: serde::de::DeserializeOwned,
    {
        retry(ExponentialBackoff::default(), || async {
            let mut client = self.client();
            Ok(client.get_bincode_cells(table, row_keys).await?)
        })
            .await
    }
}