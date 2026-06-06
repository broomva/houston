//! `/v1/workspaces` REST routes.

use crate::routes::error::ApiError;
use crate::state::ServerState;
use axum::{
    extract::{Path, State},
    routing::{delete, get, patch, post},
    Json, Router,
};
use houston_engine_core::agents_crud::{self, Agent, CreateAgent, CreateAgentResult, UpdateAgent};
use houston_engine_core::context_bootstrap::{self, ContextDraft, ImportSource, ImportSummary};
use houston_engine_core::workspace_context::{self, WorkspaceContext};
use houston_engine_core::workspaces::{self, CreateWorkspace, RenameWorkspace, Workspace};
use houston_engine_core::CoreError;
use houston_terminal_manager::Provider;
use serde::Deserialize;
use std::sync::Arc;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/workspaces", get(list).post(create))
        .route("/workspaces/:id", delete(remove))
        .route("/workspaces/:id/rename", post(rename))
        .route("/workspaces/:id/locale", patch(set_locale))
        .route("/workspaces/:id/context", get(get_context).put(put_context))
        .route("/workspaces/:id/context/import", post(import_context))
        .route(
            "/workspaces/:id/context/synthesize",
            post(synthesize_context),
        )
        // Workspace-scoped agents CRUD.
        .route(
            "/workspaces/:id/agents",
            get(list_agents).post(create_agent),
        )
        .route(
            "/workspaces/:id/agents/:agent_id",
            patch(update_agent).delete(delete_agent),
        )
        .route(
            "/workspaces/:id/agents/:agent_id/rename",
            post(rename_agent),
        )
}

async fn list(State(st): State<Arc<ServerState>>) -> Result<Json<Vec<Workspace>>, ApiError> {
    Ok(Json(workspaces::list(st.engine.paths.docs())?))
}

async fn create(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<CreateWorkspace>,
) -> Result<Json<Workspace>, ApiError> {
    Ok(Json(workspaces::create(st.engine.paths.docs(), req)?))
}

async fn remove(
    State(st): State<Arc<ServerState>>,
    Path(id): Path<String>,
) -> Result<(), ApiError> {
    workspaces::delete(st.engine.paths.docs(), &id)?;
    Ok(())
}

async fn rename(
    State(st): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Json(req): Json<RenameWorkspace>,
) -> Result<Json<Workspace>, ApiError> {
    Ok(Json(workspaces::rename(st.engine.paths.docs(), &id, req)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetWorkspaceLocale {
    /// BCP-47 base tag (`"en"` / `"es"` / `"pt"`). `null` or empty clears the
    /// per-workspace override so the workspace inherits the global `locale`.
    locale: Option<String>,
}

async fn set_locale(
    State(st): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Json(req): Json<SetWorkspaceLocale>,
) -> Result<Json<Workspace>, ApiError> {
    Ok(Json(workspaces::set_locale(
        st.engine.paths.docs(),
        &id,
        req.locale,
    )?))
}

async fn get_context(
    State(st): State<Arc<ServerState>>,
    Path(id): Path<String>,
) -> Result<Json<WorkspaceContext>, ApiError> {
    let dir = workspace_context::resolve_dir(st.engine.paths.docs(), &id)?;
    Ok(Json(workspace_context::read(&dir)?))
}

async fn put_context(
    State(st): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Json(body): Json<WorkspaceContext>,
) -> Result<Json<WorkspaceContext>, ApiError> {
    let dir = workspace_context::resolve_dir(st.engine.paths.docs(), &id)?;
    workspace_context::write(&dir, &body)?;
    Ok(Json(workspace_context::read(&dir)?))
}

/// Body for `POST /workspaces/:id/context/import`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportContextBody {
    sources: Vec<ImportSource>,
}

/// Ingest the chosen sources into a bounded, redacted corpus staged under the
/// workspace. The folder walk can be slow, so it runs on a blocking thread.
async fn import_context(
    State(st): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Json(body): Json<ImportContextBody>,
) -> Result<Json<ImportSummary>, ApiError> {
    let dir = workspace_context::resolve_dir(st.engine.paths.docs(), &id)?;
    let sources = body.sources;
    let summary = tokio::task::spawn_blocking(move || context_bootstrap::ingest(&dir, &sources))
        .await
        .map_err(|e| CoreError::Internal(format!("import task panicked: {e}")))??;
    Ok(Json(summary))
}

/// Body for `POST /workspaces/:id/context/synthesize`. Provider/model default to
/// the workspace's pinned provider (resolved client-side) or Anthropic.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesizeContextBody {
    provider: Option<String>,
    model: Option<String>,
}

/// Run the staged corpus through the provider CLI to draft USER.md + WORKSPACE.md
/// and residual questions. Persists nothing — the user reviews, then `put_context`.
async fn synthesize_context(
    State(st): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Json(body): Json<SynthesizeContextBody>,
) -> Result<Json<ContextDraft>, ApiError> {
    let dir = workspace_context::resolve_dir(st.engine.paths.docs(), &id)?;
    let provider = match body.provider.as_deref() {
        Some(p) => p.parse().map_err(CoreError::BadRequest)?,
        None => Provider::default(),
    };
    let draft = context_bootstrap::synthesize(&dir, provider, body.model.as_deref()).await?;
    Ok(Json(draft))
}

// -- Workspace-scoped agent CRUD --

async fn list_agents(
    State(st): State<Arc<ServerState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Agent>>, ApiError> {
    Ok(Json(agents_crud::list(st.engine.paths.docs(), &id)?))
}

async fn create_agent(
    State(st): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateAgent>,
) -> Result<Json<CreateAgentResult>, ApiError> {
    Ok(Json(agents_crud::create(st.engine.paths.docs(), &id, req)?))
}

async fn delete_agent(
    State(st): State<Arc<ServerState>>,
    Path((id, agent_id)): Path<(String, String)>,
) -> Result<(), ApiError> {
    agents_crud::delete(st.engine.paths.docs(), &id, &agent_id)?;
    Ok(())
}

async fn update_agent(
    State(st): State<Arc<ServerState>>,
    Path((id, agent_id)): Path<(String, String)>,
    Json(req): Json<UpdateAgent>,
) -> Result<Json<Agent>, ApiError> {
    Ok(Json(agents_crud::update(
        st.engine.paths.docs(),
        &id,
        &agent_id,
        req,
    )?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameAgentBody {
    new_name: String,
}

async fn rename_agent(
    State(st): State<Arc<ServerState>>,
    Path((id, agent_id)): Path<(String, String)>,
    Json(body): Json<RenameAgentBody>,
) -> Result<Json<Agent>, ApiError> {
    Ok(Json(agents_crud::rename(
        st.engine.paths.docs(),
        &id,
        &agent_id,
        &body.new_name,
    )?))
}
