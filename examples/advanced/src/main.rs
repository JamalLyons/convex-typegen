//! Smoke test: seeds demo data (if needed), then calls queries, mutations, and an action
//! using every generated `*Args` struct from this example’s Convex modules.

mod convex_types;

use std::collections::BTreeMap;
use std::path::Path;

use convex::{ConvexClient, FunctionResult, Value as ConvexValue};
use convex_typegen::prelude::*;
use convex_types::{
    IntegrationsMirrorArgs, ProjectsListByTeamArgs, ProjectsUpdateTagsArgs, TasksCreateArgs, TasksSearchArgs,
    TeamsListByOwnerArgs, UsersCreateArgs, UsersGetByEmailArgs, UsersGetProfileArgs, WorkspaceSeedIfEmptyArgs,
    WorkspaceSummaryArgs,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::from_filename(Path::new(env!("CARGO_MANIFEST_DIR")).join(".env.local")).ok();

    let Ok(url) = std::env::var("CONVEX_URL") else {
        eprintln!("CONVEX_URL is not set. Add it to .env.local next to this crate’s Cargo.toml.");
        eprintln!("Example paths from codegen:");
        eprintln!("  {}", IntegrationsMirrorArgs::FUNCTION_PATH);
        eprintln!("  {}", ProjectsListByTeamArgs::FUNCTION_PATH);
        eprintln!("  {}", TasksSearchArgs::FUNCTION_PATH);
        eprintln!("  {}", WorkspaceSummaryArgs::FUNCTION_PATH);
        return Ok(());
    };

    let mut client = ConvexClient::new(&url).await?;

    let seed = client
        .mutation(
            WorkspaceSeedIfEmptyArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(WorkspaceSeedIfEmptyArgs {})?,
        )
        .await?;
    println!("workspaceSeedIfEmpty → {:?}", seed);

    let summary = client
        .query(
            WorkspaceSummaryArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(WorkspaceSummaryArgs {})?,
        )
        .await?;
    println!("workspaceSummary → {:?}", summary);

    let (team_id, project_id, user_id) = extract_ids(&summary);
    let (Some(team_id), Some(project_id), Some(user_id)) = (team_id, project_id, user_id) else {
        println!("Not enough data in deployment yet (need at least one user, team, and project).");
        return Ok(());
    };

    let teams = client
        .query(
            TeamsListByOwnerArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(TeamsListByOwnerArgs {
                ownerUserId: user_id.clone(),
            })?,
        )
        .await?;
    println!("teamsListByOwner → {:?}", teams);

    let projects = client
        .query(
            ProjectsListByTeamArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(ProjectsListByTeamArgs {
                teamId: team_id.clone(),
                statusFilter: Some(convex_typegen::serde_json::json!("active")),
            })?,
        )
        .await?;
    println!("projectsListByTeam → {:?}", projects);

    let tasks = client
        .query(
            TasksSearchArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(TasksSearchArgs {
                filter: convex_typegen::serde_json::json!({
                    "projectId": project_id,
                    "minPriority": "p1",
                }),
                limit: Some(10.0),
            })?,
        )
        .await?;
    println!("tasksSearch → {:?}", tasks);

    let mirror = client
        .action(
            IntegrationsMirrorArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(IntegrationsMirrorArgs {
                body: "hello from Rust".to_string(),
                numbers: vec![1.0, 2.0, 3.0],
                flags: BTreeMap::from([("verbose".to_string(), true), ("trace".to_string(), false)]),
                mode: convex_typegen::serde_json::json!("json"),
                extra: Some(std::collections::HashMap::from([("k".to_string(), "v".to_string())])),
            })?,
        )
        .await?;
    println!("integrationsMirror → {:?}", mirror);

    let profile = client
        .query(
            UsersGetProfileArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(UsersGetProfileArgs {
                userId: user_id.clone(),
                withBytes: None,
            })?,
        )
        .await?;
    println!("usersGetProfile → {:?}", profile);

    let by_email = client
        .query(
            UsersGetByEmailArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(UsersGetByEmailArgs {
                email: "demo@example.com".to_string(),
                includeInactive: None,
            })?,
        )
        .await?;
    println!("usersGetByEmail → {:?}", by_email);

    let _tag_update = client
        .mutation(
            ProjectsUpdateTagsArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(ProjectsUpdateTagsArgs {
                projectId: project_id.clone(),
                tags: vec!["rust".to_string(), "convex".to_string(), "typegen".to_string()],
            })?,
        )
        .await?;
    println!("projectsUpdateTags ok");

    let _task = client
        .mutation(
            TasksCreateArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(TasksCreateArgs {
                projectId: project_id.clone(),
                title: "From Rust client".to_string(),
                priority: convex_typegen::serde_json::json!("p2"),
                assigneeUserId: Some(user_id),
                payload: Some(convex_typegen::serde_json::json!({ "source": "advanced example" })),
                dueAt: None,
            })?,
        )
        .await?;
    println!("tasksCreate ok");

    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let _alt_user = client
        .mutation(
            UsersCreateArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(UsersCreateArgs {
                email: format!("rust-{ts}@example.com"),
                displayName: "Rust-created".to_string(),
                role: convex_typegen::serde_json::json!("member"),
                metadata: Some(std::collections::HashMap::from([(
                    "channel".to_string(),
                    "advanced-example".to_string(),
                )])),
                score: Some(7),
                avatarBytes: None,
            })?,
        )
        .await;
    println!("usersCreate (extra) → {:?}", _alt_user);

    Ok(())
}

fn extract_ids(summary: &FunctionResult) -> (Option<String>, Option<String>, Option<String>) {
    let FunctionResult::Value(ConvexValue::Object(obj)) = summary else {
        return (None, None, None);
    };
    let team = obj.get("firstTeamId").and_then(string_from_convex_value);
    let project = obj.get("firstProjectId").and_then(string_from_convex_value);
    let user = obj.get("firstUserId").and_then(string_from_convex_value);
    (team, project, user)
}

fn string_from_convex_value(v: &ConvexValue) -> Option<String> {
    match v {
        ConvexValue::String(s) => Some(s.clone()),
        _ => None,
    }
}
