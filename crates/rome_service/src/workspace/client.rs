use std::{
    panic::RefUnwindSafe,
    sync::atomic::{AtomicU64, Ordering},
};

use rome_formatter::Printed;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;

use crate::workspace::{RageParams, RageResult, ServerInfo, SupportsFeatureResult};
use crate::{RomeError, TransportError, Workspace};

use super::{
    ChangeFileParams, CloseFileParams, FixFileParams, FixFileResult, FormatFileParams,
    FormatOnTypeParams, FormatRangeParams, GetControlFlowGraphParams, GetFormatterIRParams,
    GetSyntaxTreeParams, GetSyntaxTreeResult, OpenFileParams, PullActionsParams, PullActionsResult,
    PullDiagnosticsParams, PullDiagnosticsResult, RenameParams, RenameResult,
    SupportsFeatureParams, UpdateSettingsParams,
};

pub struct WorkspaceClient<T> {
    transport: T,
    request_id: AtomicU64,
    server_info: Option<ServerInfo>,
}

pub trait WorkspaceTransport {
    fn request<P, R>(&self, request: TransportRequest<P>) -> Result<R, TransportError>
    where
        P: Serialize,
        R: DeserializeOwned;
}

#[derive(Debug)]
pub struct TransportRequest<P> {
    pub id: u64,
    pub method: &'static str,
    pub params: P,
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Information about the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<ServerInfo>,
}

impl<T> WorkspaceClient<T>
where
    T: WorkspaceTransport + RefUnwindSafe + Send + Sync,
{
    pub fn new(transport: T) -> Result<Self, RomeError> {
        let mut client = Self {
            transport,
            request_id: AtomicU64::new(0),
            server_info: None,
        };

        // TODO: The current implementation of the JSON-RPC protocol in
        // tower_lsp doesn't allow any request to be sent before a call to
        // initialize, this is something we could be able to lift by using our
        // own RPC protocol implementation
        let value: InitializeResult = client.request(
            "initialize",
            json!({
                "capabilities": {},
                "clientInfo": {
                    "name": env!("CARGO_PKG_NAME"),
                    "version": crate::VERSION
                },
            }),
        )?;

        client.server_info = value.server_info;

        Ok(client)
    }

    fn request<P, R>(&self, method: &'static str, params: P) -> Result<R, RomeError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let request = TransportRequest { id, method, params };

        let response = self.transport.request(request)?;

        Ok(response)
    }

    pub fn shutdown(self) -> Result<(), RomeError> {
        self.request("rome/shutdown", ())
    }
}

impl<T> Workspace for WorkspaceClient<T>
where
    T: WorkspaceTransport + RefUnwindSafe + Send + Sync,
{
    fn supports_feature(
        &self,
        params: SupportsFeatureParams,
    ) -> Result<SupportsFeatureResult, RomeError> {
        self.request("rome/supports_feature", params)
    }

    fn update_settings(&self, params: UpdateSettingsParams) -> Result<(), RomeError> {
        self.request("rome/update_settings", params)
    }

    fn open_file(&self, params: OpenFileParams) -> Result<(), RomeError> {
        self.request("rome/open_file", params)
    }

    fn get_syntax_tree(
        &self,
        params: GetSyntaxTreeParams,
    ) -> Result<GetSyntaxTreeResult, RomeError> {
        self.request("rome/get_syntax_tree", params)
    }

    fn get_control_flow_graph(
        &self,
        params: GetControlFlowGraphParams,
    ) -> Result<String, RomeError> {
        self.request("rome/get_control_flow_graph", params)
    }

    fn get_formatter_ir(&self, params: GetFormatterIRParams) -> Result<String, RomeError> {
        self.request("rome/get_formatter_ir", params)
    }

    fn change_file(&self, params: ChangeFileParams) -> Result<(), RomeError> {
        self.request("rome/change_file", params)
    }

    fn close_file(&self, params: CloseFileParams) -> Result<(), RomeError> {
        self.request("rome/close_file", params)
    }

    fn pull_diagnostics(
        &self,
        params: PullDiagnosticsParams,
    ) -> Result<PullDiagnosticsResult, RomeError> {
        self.request("rome/pull_diagnostics", params)
    }

    fn pull_actions(&self, params: PullActionsParams) -> Result<PullActionsResult, RomeError> {
        self.request("rome/pull_actions", params)
    }

    fn format_file(&self, params: FormatFileParams) -> Result<Printed, RomeError> {
        self.request("rome/format_file", params)
    }

    fn format_range(&self, params: FormatRangeParams) -> Result<Printed, RomeError> {
        self.request("rome/format_range", params)
    }

    fn format_on_type(&self, params: FormatOnTypeParams) -> Result<Printed, RomeError> {
        self.request("rome/format_on_type", params)
    }

    fn fix_file(&self, params: FixFileParams) -> Result<FixFileResult, RomeError> {
        self.request("rome/fix_file", params)
    }

    fn rename(&self, params: RenameParams) -> Result<RenameResult, RomeError> {
        self.request("rome/rename", params)
    }

    fn rage(&self, params: RageParams) -> Result<RageResult, RomeError> {
        self.request("rome/rage", params)
    }

    fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }
}
