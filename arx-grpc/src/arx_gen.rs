/// Pre-generated protobuf + tonic types for arx.proto.
/// Regenerate with: protoc installed + `cargo build -p arx-grpc`
///
/// In tonic's ProstCodec<Encode, Decode>:
///   - Encode = what the server *sends* back to the client (response)
///   - Decode = what the server *receives* from the client (request)
use tonic::codec::ProstCodec;

// ── Key material ──────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct KeyMaterial {
    #[prost(oneof = "key_material::KeySource", tags = "1, 2")]
    pub key_source: ::core::option::Option<key_material::KeySource>,
}
pub mod key_material {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum KeySource {
        #[prost(bytes, tag = "1")]   RawKey(::prost::alloc::vec::Vec<u8>),
        #[prost(string, tag = "2")]  Password(::prost::alloc::string::String),
    }
}

// ── Upload frames ─────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackHeader {
    #[prost(string, tag = "1")] pub archive_name:  String,
    #[prost(bool,   tag = "2")] pub deterministic: bool,
    #[prost(float,  tag = "3")] pub min_gain:      f32,
    #[prost(message, optional, tag = "4")] pub key: Option<KeyMaterial>,
    #[prost(string, tag = "5")] pub label: String,
    #[prost(string, tag = "6")] pub owner: String,
    #[prost(string, tag = "7")] pub notes: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudHeader {
    #[prost(string, tag = "1")] pub archive_id: String,
    #[prost(message, optional, tag = "2")] pub key: Option<KeyMaterial>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FileInfo {
    #[prost(string, tag = "1")] pub path:  String,
    #[prost(uint32, tag = "2")] pub mode:  u32,
    #[prost(int64,  tag = "3")] pub mtime: i64,
    #[prost(uint64, tag = "4")] pub size:  u64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UploadFrame {
    #[prost(oneof = "upload_frame::Payload", tags = "1, 2, 3, 4, 5")]
    pub payload: ::core::option::Option<upload_frame::Payload>,
}
pub mod upload_frame {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Payload {
        #[prost(message, tag = "1")] PackHeader(super::PackHeader),
        #[prost(message, tag = "2")] CrudHeader(super::CrudHeader),
        #[prost(message, tag = "3")] FileInfo(super::FileInfo),
        #[prost(bytes,   tag = "4")] Data(::prost::alloc::vec::Vec<u8>),
        #[prost(bool,    tag = "5")] Finalize(bool),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DownloadFrame {
    #[prost(oneof = "download_frame::Payload", tags = "1, 2, 3")]
    pub payload: ::core::option::Option<download_frame::Payload>,
}
pub mod download_frame {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Payload {
        #[prost(message, tag = "1")] File(super::FileInfo),
        #[prost(bytes,   tag = "2")] Data(::prost::alloc::vec::Vec<u8>),
        #[prost(string,  tag = "3")] Error(::prost::alloc::string::String),
    }
}

// ── Messages ──────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ArchiveStats {
    #[prost(uint64, tag = "1")] pub files: u64,
    #[prost(uint64, tag = "2")] pub dirs: u64,
    #[prost(uint64, tag = "3")] pub chunks: u64,
    #[prost(uint64, tag = "4")] pub logical_bytes: u64,
    #[prost(uint64, tag = "5")] pub stored_bytes: u64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackResponse {
    #[prost(string, tag = "1")] pub archive_id: String,
    #[prost(message, optional, tag = "2")] pub stats: Option<ArchiveStats>,
    #[prost(string, tag = "3")] pub error: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListRequest { #[prost(string, tag="1")] pub archive_id: String, #[prost(message, optional, tag="2")] pub key: Option<KeyMaterial> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListEntry { #[prost(string, tag="1")] pub path: String, #[prost(uint64, tag="2")] pub u_size: u64, #[prost(uint64, tag="3")] pub c_size: u64, #[prost(uint32, tag="4")] pub chunks: u32 }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListResponse { #[prost(message, repeated, tag="1")] pub entries: Vec<ListEntry> }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExtractRequest { #[prost(string, tag="1")] pub archive_id: String, #[prost(string, tag="2")] pub path: String, #[prost(uint64, tag="3")] pub start: u64, #[prost(uint64, tag="4")] pub len: u64, #[prost(message, optional, tag="5")] pub key: Option<KeyMaterial> }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VerifyRequest  { #[prost(string, tag="1")] pub archive_id: String, #[prost(message, optional, tag="2")] pub key: Option<KeyMaterial> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VerifyResponse { #[prost(bool, tag="1")] pub ok: bool, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IssueRequest { #[prost(string, tag="1")] pub archive_name: String, #[prost(string, tag="2")] pub label: String, #[prost(string, tag="3")] pub owner: String, #[prost(string, tag="4")] pub notes: String, #[prost(bool, tag="5")] pub deterministic: bool, #[prost(message, optional, tag="6")] pub key: Option<KeyMaterial> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IssueResponse { #[prost(string, tag="1")] pub archive_id: String, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudAddResponse { #[prost(bool, tag="1")] pub ok: bool, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudRmRequest  { #[prost(string, tag="1")] pub archive_id: String, #[prost(string, tag="2")] pub path: String, #[prost(bool, tag="3")] pub recursive: bool, #[prost(message, optional, tag="4")] pub key: Option<KeyMaterial> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudRmResponse { #[prost(bool, tag="1")] pub ok: bool, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudMvRequest  { #[prost(string, tag="1")] pub archive_id: String, #[prost(string, tag="2")] pub from: String, #[prost(string, tag="3")] pub to: String, #[prost(message, optional, tag="4")] pub key: Option<KeyMaterial> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudMvResponse { #[prost(bool, tag="1")] pub ok: bool, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudLsRequest  { #[prost(string, tag="1")] pub archive_id: String, #[prost(string, tag="2")] pub prefix: String, #[prost(bool, tag="3")] pub long_format: bool, #[prost(message, optional, tag="4")] pub key: Option<KeyMaterial> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudLsEntry    { #[prost(string, tag="1")] pub path: String, #[prost(uint64, tag="2")] pub size: u64, #[prost(uint64, tag="3")] pub mtime: u64 }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudLsResponse { #[prost(message, repeated, tag="1")] pub entries: Vec<CrudLsEntry> }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudSyncRequest  { #[prost(string, tag="1")] pub archive_id: String, #[prost(bool, tag="2")] pub seal_base: bool, #[prost(message, optional, tag="3")] pub key: Option<KeyMaterial> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudSyncResponse { #[prost(string, tag="1")] pub new_archive_id: String, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudDiffEntry    { #[prost(string, tag="1")] pub kind: String, #[prost(string, tag="2")] pub path: String }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudDiffRequest  { #[prost(string, tag="1")] pub archive_id: String, #[prost(message, optional, tag="2")] pub key: Option<KeyMaterial> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CrudDiffResponse { #[prost(message, repeated, tag="1")] pub entries: Vec<CrudDiffEntry> }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChunkMapRequest  { #[prost(string, tag="1")] pub archive_id: String, #[prost(string, tag="2")] pub path: String, #[prost(message, optional, tag="3")] pub key: Option<KeyMaterial> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChunkMapEntry    { #[prost(uint64, tag="1")] pub ordinal: u64, #[prost(uint64, tag="2")] pub id: u64, #[prost(uint32, tag="3")] pub codec: u32, #[prost(uint64, tag="4")] pub u_len: u64, #[prost(uint64, tag="5")] pub c_len: u64, #[prost(uint64, tag="6")] pub data_off: u64 }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChunkMapResponse { #[prost(message, repeated, tag="1")] pub entries: Vec<ChunkMapEntry> }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ArchiveInfo      { #[prost(string, tag="1")] pub id: String, #[prost(string, tag="2")] pub name: String, #[prost(uint64, tag="3")] pub size_bytes: u64, #[prost(string, tag="4")] pub created_at: String, #[prost(bool, tag="5")] pub encrypted: bool }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListArchivesReq  {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListArchivesResp { #[prost(message, repeated, tag="1")] pub archives: Vec<ArchiveInfo> }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteArchiveReq  { #[prost(string, tag="1")] pub archive_id: String }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteArchiveResp { #[prost(bool, tag="1")] pub ok: bool, #[prost(string, tag="2")] pub error: String }

// ── Auth messages ─────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginRequest { #[prost(string, tag="1")] pub email: String, #[prost(string, tag="2")] pub password: String }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginResponse { #[prost(string, tag="1")] pub access_token: String, #[prost(string, tag="2")] pub refresh_token: String, #[prost(uint32, tag="3")] pub expires_in: u32, #[prost(string, tag="4")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RefreshTokenRequest { #[prost(string, tag="1")] pub refresh_token: String }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RefreshTokenResponse { #[prost(string, tag="1")] pub access_token: String, #[prost(string, tag="2")] pub new_refresh_token: String, #[prost(uint32, tag="3")] pub expires_in: u32, #[prost(string, tag="4")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LogoutRequest { #[prost(string, tag="1")] pub refresh_token: String }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LogoutResponse { #[prost(bool, tag="1")] pub ok: bool, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WhoamiRequest {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WhoamiResponse { #[prost(string, tag="1")] pub user_id: String, #[prost(string, tag="2")] pub email: String, #[prost(string, tag="3")] pub tenant_id: String }

// ── Admin messages ────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateTenantRequest { #[prost(string, tag="1")] pub name: String }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateTenantResponse { #[prost(string, tag="1")] pub tenant_id: String, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateUserRequest { #[prost(string, tag="1")] pub tenant_id: String, #[prost(string, tag="2")] pub email: String, #[prost(string, tag="3")] pub password: String }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateUserResponse { #[prost(string, tag="1")] pub user_id: String, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateApiKeyRequest { #[prost(string, tag="1")] pub user_id: String, #[prost(string, tag="2")] pub name: String, #[prost(uint64, tag="3")] pub expires_in_secs: u64 }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateApiKeyResponse { #[prost(string, tag="1")] pub api_key: String, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RevokeApiKeyRequest { #[prost(string, tag="1")] pub key_hash: String }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RevokeApiKeyResponse { #[prost(bool, tag="1")] pub ok: bool, #[prost(string, tag="2")] pub error: String }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTenantsRequest {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TenantInfoMsg { #[prost(string, tag="1")] pub id: String, #[prost(string, tag="2")] pub name: String, #[prost(int64, tag="3")] pub created_at: i64 }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTenantsResponse { #[prost(message, repeated, tag="1")] pub tenants: Vec<TenantInfoMsg> }

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListUsersRequest { #[prost(string, tag="1")] pub tenant_id: String }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UserInfoMsg { #[prost(string, tag="1")] pub id: String, #[prost(string, tag="2")] pub email: String, #[prost(string, tag="3")] pub tenant_id: String, #[prost(bool, tag="4")] pub active: bool, #[prost(int64, tag="5")] pub created_at: i64 }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListUsersResponse { #[prost(message, repeated, tag="1")] pub users: Vec<UserInfoMsg> }

// ── Service trait ─────────────────────────────────────────────────────────────

#[tonic::async_trait]
pub trait ArxService: Send + Sync + 'static {
    async fn pack_stream(
        &self, request: tonic::Request<tonic::Streaming<UploadFrame>>,
    ) -> std::result::Result<tonic::Response<PackResponse>, tonic::Status>;

    async fn list(
        &self, request: tonic::Request<ListRequest>,
    ) -> std::result::Result<tonic::Response<ListResponse>, tonic::Status>;

    type ExtractStreamStream: futures_core::Stream<
        Item = std::result::Result<DownloadFrame, tonic::Status>>
        + Send + 'static;
    async fn extract_stream(
        &self, request: tonic::Request<ExtractRequest>,
    ) -> std::result::Result<tonic::Response<Self::ExtractStreamStream>, tonic::Status>;

    async fn verify(
        &self, request: tonic::Request<VerifyRequest>,
    ) -> std::result::Result<tonic::Response<VerifyResponse>, tonic::Status>;

    async fn issue(
        &self, request: tonic::Request<IssueRequest>,
    ) -> std::result::Result<tonic::Response<IssueResponse>, tonic::Status>;

    async fn crud_add_stream(
        &self, request: tonic::Request<tonic::Streaming<UploadFrame>>,
    ) -> std::result::Result<tonic::Response<CrudAddResponse>, tonic::Status>;

    async fn crud_rm(
        &self, request: tonic::Request<CrudRmRequest>,
    ) -> std::result::Result<tonic::Response<CrudRmResponse>, tonic::Status>;

    async fn crud_mv(
        &self, request: tonic::Request<CrudMvRequest>,
    ) -> std::result::Result<tonic::Response<CrudMvResponse>, tonic::Status>;

    async fn crud_ls(
        &self, request: tonic::Request<CrudLsRequest>,
    ) -> std::result::Result<tonic::Response<CrudLsResponse>, tonic::Status>;

    async fn crud_sync(
        &self, request: tonic::Request<CrudSyncRequest>,
    ) -> std::result::Result<tonic::Response<CrudSyncResponse>, tonic::Status>;

    async fn crud_diff(
        &self, request: tonic::Request<CrudDiffRequest>,
    ) -> std::result::Result<tonic::Response<CrudDiffResponse>, tonic::Status>;

    async fn chunk_map(
        &self, request: tonic::Request<ChunkMapRequest>,
    ) -> std::result::Result<tonic::Response<ChunkMapResponse>, tonic::Status>;

    async fn list_archives(
        &self, request: tonic::Request<ListArchivesReq>,
    ) -> std::result::Result<tonic::Response<ListArchivesResp>, tonic::Status>;

    async fn delete_archive(
        &self, request: tonic::Request<DeleteArchiveReq>,
    ) -> std::result::Result<tonic::Response<DeleteArchiveResp>, tonic::Status>;

    // ── Auth RPCs ──────────────────────────────────────────────────────────────
    async fn login(
        &self, request: tonic::Request<LoginRequest>,
    ) -> std::result::Result<tonic::Response<LoginResponse>, tonic::Status>;

    async fn refresh_token(
        &self, request: tonic::Request<RefreshTokenRequest>,
    ) -> std::result::Result<tonic::Response<RefreshTokenResponse>, tonic::Status>;

    async fn logout(
        &self, request: tonic::Request<LogoutRequest>,
    ) -> std::result::Result<tonic::Response<LogoutResponse>, tonic::Status>;

    async fn whoami(
        &self, request: tonic::Request<WhoamiRequest>,
    ) -> std::result::Result<tonic::Response<WhoamiResponse>, tonic::Status>;

    // ── Admin RPCs ─────────────────────────────────────────────────────────────
    async fn create_tenant(
        &self, request: tonic::Request<CreateTenantRequest>,
    ) -> std::result::Result<tonic::Response<CreateTenantResponse>, tonic::Status>;

    async fn create_user(
        &self, request: tonic::Request<CreateUserRequest>,
    ) -> std::result::Result<tonic::Response<CreateUserResponse>, tonic::Status>;

    async fn create_api_key(
        &self, request: tonic::Request<CreateApiKeyRequest>,
    ) -> std::result::Result<tonic::Response<CreateApiKeyResponse>, tonic::Status>;

    async fn revoke_api_key(
        &self, request: tonic::Request<RevokeApiKeyRequest>,
    ) -> std::result::Result<tonic::Response<RevokeApiKeyResponse>, tonic::Status>;

    async fn list_tenants(
        &self, request: tonic::Request<ListTenantsRequest>,
    ) -> std::result::Result<tonic::Response<ListTenantsResponse>, tonic::Status>;

    async fn list_users(
        &self, request: tonic::Request<ListUsersRequest>,
    ) -> std::result::Result<tonic::Response<ListUsersResponse>, tonic::Status>;
}

// ── Server wrapper ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ArxServiceServer<T: ArxService> {
    inner: std::sync::Arc<T>,
    accept_compression_encodings: tonic::codec::EnabledCompressionEncodings,
    send_compression_encodings: tonic::codec::EnabledCompressionEncodings,
    max_decoding_message_size: Option<usize>,
    max_encoding_message_size: Option<usize>,
}

impl<T: ArxService> ArxServiceServer<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: std::sync::Arc::new(inner),
            accept_compression_encodings: Default::default(),
            send_compression_encodings: Default::default(),
            max_decoding_message_size: None,
            max_encoding_message_size: None,
        }
    }
    pub fn with_interceptor<F>(inner: T, interceptor: F)
        -> tonic::codegen::InterceptedService<Self, F>
    where
        F: tonic::service::Interceptor,
    {
        tonic::codegen::InterceptedService::new(Self::new(inner), interceptor)
    }
}

impl<T, B> tonic::codegen::Service<http::Request<B>> for ArxServiceServer<T>
where
    T: ArxService,
    B: tonic::codegen::Body + Send + 'static,
    B::Error: Into<tonic::codegen::StdError> + Send + 'static,
{
    type Response = http::Response<tonic::body::BoxBody>;
    type Error = std::convert::Infallible;
    type Future = tonic::codegen::BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>)
        -> std::task::Poll<std::result::Result<(), Self::Error>>
    { std::task::Poll::Ready(Ok(())) }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        let inner = self.inner.clone();
        let max_dec = self.max_decoding_message_size;
        let max_enc = self.max_encoding_message_size;

        // Helper macro to reduce boilerplate for unary RPCs
        macro_rules! unary_handler {
            ($req_ty:ty, $resp_ty:ty, $method:ident) => {{
                #[allow(non_camel_case_types)]
                struct Svc<T: ArxService>(std::sync::Arc<T>);
                impl<T: ArxService> tonic::server::UnaryService<$req_ty> for Svc<T> {
                    type Response = $resp_ty;
                    type Future = tonic::codegen::BoxFuture<
                        tonic::Response<Self::Response>, tonic::Status>;
                    fn call(&mut self, req: tonic::Request<$req_ty>) -> Self::Future {
                        let i = self.0.clone();
                        Box::pin(async move { i.$method(req).await })
                    }
                }
                let fut = async move {
                    let svc = Svc(inner);
                    // ProstCodec<Encode=Response, Decode=Request>
                    let codec = ProstCodec::<$resp_ty, $req_ty>::default();
                    let mut grpc = tonic::server::Grpc::new(codec);
                    if let Some(sz) = max_dec { grpc = grpc.max_decoding_message_size(sz); }
                    if let Some(sz) = max_enc { grpc = grpc.max_encoding_message_size(sz); }
                    Ok(grpc.unary(svc, req).await)
                };
                Box::pin(fut)
            }};
        }

        macro_rules! client_streaming_handler {
            ($req_ty:ty, $resp_ty:ty, $method:ident) => {{
                #[allow(non_camel_case_types)]
                struct Svc<T: ArxService>(std::sync::Arc<T>);
                impl<T: ArxService> tonic::server::ClientStreamingService<$req_ty> for Svc<T> {
                    type Response = $resp_ty;
                    type Future = tonic::codegen::BoxFuture<
                        tonic::Response<Self::Response>, tonic::Status>;
                    fn call(&mut self, req: tonic::Request<tonic::Streaming<$req_ty>>) -> Self::Future {
                        let i = self.0.clone();
                        Box::pin(async move { i.$method(req).await })
                    }
                }
                let fut = async move {
                    let svc = Svc(inner);
                    let codec = ProstCodec::<$resp_ty, $req_ty>::default();
                    let mut grpc = tonic::server::Grpc::new(codec);
                    if let Some(sz) = max_dec { grpc = grpc.max_decoding_message_size(sz); }
                    if let Some(sz) = max_enc { grpc = grpc.max_encoding_message_size(sz); }
                    Ok(grpc.client_streaming(svc, req).await)
                };
                Box::pin(fut)
            }};
        }

        match req.uri().path() {
            "/arx.ArxService/PackStream"    => client_streaming_handler!(UploadFrame, PackResponse, pack_stream),
            "/arx.ArxService/CrudAddStream" => client_streaming_handler!(UploadFrame, CrudAddResponse, crud_add_stream),
            "/arx.ArxService/List"          => unary_handler!(ListRequest, ListResponse, list),
            "/arx.ArxService/Verify"        => unary_handler!(VerifyRequest, VerifyResponse, verify),
            "/arx.ArxService/Issue"         => unary_handler!(IssueRequest, IssueResponse, issue),
            "/arx.ArxService/CrudRm"        => unary_handler!(CrudRmRequest, CrudRmResponse, crud_rm),
            "/arx.ArxService/CrudMv"        => unary_handler!(CrudMvRequest, CrudMvResponse, crud_mv),
            "/arx.ArxService/CrudLs"        => unary_handler!(CrudLsRequest, CrudLsResponse, crud_ls),
            "/arx.ArxService/CrudSync"      => unary_handler!(CrudSyncRequest, CrudSyncResponse, crud_sync),
            "/arx.ArxService/CrudDiff"      => unary_handler!(CrudDiffRequest, CrudDiffResponse, crud_diff),
            "/arx.ArxService/ChunkMap"      => unary_handler!(ChunkMapRequest, ChunkMapResponse, chunk_map),
            "/arx.ArxService/ListArchives"  => unary_handler!(ListArchivesReq, ListArchivesResp, list_archives),
            "/arx.ArxService/DeleteArchive" => unary_handler!(DeleteArchiveReq, DeleteArchiveResp, delete_archive),
            "/arx.ArxService/Login"         => unary_handler!(LoginRequest, LoginResponse, login),
            "/arx.ArxService/RefreshToken"  => unary_handler!(RefreshTokenRequest, RefreshTokenResponse, refresh_token),
            "/arx.ArxService/Logout"        => unary_handler!(LogoutRequest, LogoutResponse, logout),
            "/arx.ArxService/Whoami"        => unary_handler!(WhoamiRequest, WhoamiResponse, whoami),
            "/arx.ArxService/CreateTenant"  => unary_handler!(CreateTenantRequest, CreateTenantResponse, create_tenant),
            "/arx.ArxService/CreateUser"    => unary_handler!(CreateUserRequest, CreateUserResponse, create_user),
            "/arx.ArxService/CreateApiKey"  => unary_handler!(CreateApiKeyRequest, CreateApiKeyResponse, create_api_key),
            "/arx.ArxService/RevokeApiKey"  => unary_handler!(RevokeApiKeyRequest, RevokeApiKeyResponse, revoke_api_key),
            "/arx.ArxService/ListTenants"   => unary_handler!(ListTenantsRequest, ListTenantsResponse, list_tenants),
            "/arx.ArxService/ListUsers"     => unary_handler!(ListUsersRequest, ListUsersResponse, list_users),
            "/arx.ArxService/ExtractStream" => {
                #[allow(non_camel_case_types)]
                struct Svc<T: ArxService>(std::sync::Arc<T>);
                impl<T: ArxService> tonic::server::ServerStreamingService<ExtractRequest> for Svc<T> {
                    type Response = DownloadFrame;
                    type ResponseStream = T::ExtractStreamStream;
                    type Future = tonic::codegen::BoxFuture<
                        tonic::Response<Self::ResponseStream>, tonic::Status>;
                    fn call(&mut self, req: tonic::Request<ExtractRequest>) -> Self::Future {
                        let i = self.0.clone();
                        Box::pin(async move { i.extract_stream(req).await })
                    }
                }
                let fut = async move {
                    let svc = Svc(inner);
                    let codec = ProstCodec::<DownloadFrame, ExtractRequest>::default();
                    let mut grpc = tonic::server::Grpc::new(codec);
                    if let Some(sz) = max_dec { grpc = grpc.max_decoding_message_size(sz); }
                    if let Some(sz) = max_enc { grpc = grpc.max_encoding_message_size(sz); }
                    Ok(grpc.server_streaming(svc, req).await)
                };
                Box::pin(fut)
            }
            _ => Box::pin(async {
                Ok(http::Response::builder()
                    .status(200)
                    .header("grpc-status", "12")
                    .header("content-type", "application/grpc")
                    .body(tonic::body::empty_body())
                    .unwrap())
            }),
        }
    }
}

// Manual Clone impl — does not require T: Clone since inner is Arc<T>
impl<T: ArxService> Clone for ArxServiceServer<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            accept_compression_encodings: self.accept_compression_encodings,
            send_compression_encodings: self.send_compression_encodings,
            max_decoding_message_size: self.max_decoding_message_size,
            max_encoding_message_size: self.max_encoding_message_size,
        }
    }
}

impl<T: ArxService> tonic::server::NamedService for ArxServiceServer<T> {
    const NAME: &'static str = "arx.ArxService";
}

pub mod arx_service_server {
    pub use super::{ArxService, ArxServiceServer};
}
