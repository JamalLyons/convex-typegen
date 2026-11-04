//! Smoke test: seeds demo data (if needed), then calls queries, mutations, and an action
//! using every generated `*Args` struct from this example’s Convex modules.

mod convex_types;

use std::path::Path;

use convex::{ConvexClient, FunctionResult, Value as ConvexValue};
use convex_typegen_runtime::model::RawValue;
use convex_typegen_runtime::prelude::*;
use convex_types::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    dotenvy::from_filename(Path::new(env!("CARGO_MANIFEST_DIR")).join(".env.local")).ok();

    let Ok(url) = std::env::var("CONVEX_URL") else {
        eprintln!("CONVEX_URL is not set. Add it to .env.local next to this crate’s Cargo.toml.");
        eprintln!("Example paths from codegen:");
        eprintln!("  {}", IntegrationsIntegrationsMirrorArgs::FUNCTION_PATH);
        eprintln!("  {}", ProjectsProjectsListByTeamArgs::FUNCTION_PATH);
        eprintln!("  {}", TasksTasksSearchArgs::FUNCTION_PATH);
        eprintln!("  {}", WorkspaceWorkspaceSummaryArgs::FUNCTION_PATH);
        return Ok(());
    };

    let mut client = ConvexClient::new(&url).await?;

    let seed = client
        .mutation(
            WorkspaceWorkspaceSeedIfEmptyArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(WorkspaceWorkspaceSeedIfEmptyArgs(RawValue(ConvexValue::Object(
                Default::default(),
            ))))?,
        )
        .await?;
    println!("workspaceSeedIfEmpty → {:?}", seed);

    let summary = client
        .query(
            WorkspaceWorkspaceSummaryArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(WorkspaceWorkspaceSummaryArgs(RawValue(ConvexValue::Object(
                Default::default(),
            ))))?,
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
            TeamsTeamsListByOwnerArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(TeamsTeamsListByOwnerArgs {
                owner_user_id: user_id.clone().into(),
            })?,
        )
        .await?;
    println!("teamsListByOwner → {:?}", teams);

    let projects = client
        .query(
            ProjectsProjectsListByTeamArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(ProjectsProjectsListByTeamArgs {
                team_id: team_id.into(),
                status_filter: Some(ProjectsProjectsListByTeamArgsStatusFilter::Branch2(
                    ProjectsProjectsListByTeamArgsStatusFilterBranch2,
                )),
            })?,
        )
        .await?;
    println!("projectsListByTeam → {:?}", projects);

    let tasks = client
        .query(
            TasksTasksSearchArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(TasksTasksSearchArgs {
                filter: TasksTasksSearchArgsFilter {
                    project_id: project_id.clone().into(),
                    min_priority: Some(TasksTasksSearchArgsFilterMinPriority::Branch2(
                        TasksTasksSearchArgsFilterMinPriorityBranch2,
                    )),
                },
                limit: Some(10.0),
            })?,
        )
        .await?;
    println!("tasksSearch → {:?}", tasks);

    let mirror = client
        .action(
            IntegrationsIntegrationsMirrorArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(IntegrationsIntegrationsMirrorArgs {
                body: "hello from Rust".to_string(),
                numbers: vec![1.0, 2.0, 3.0],
                flags: IntegrationsIntegrationsMirrorArgsFlags {
                    verbose: true,
                    trace: false,
                },
                mode: IntegrationsIntegrationsMirrorArgsMode::Branch1(IntegrationsIntegrationsMirrorArgsModeBranch1),
                extra: Some(std::collections::BTreeMap::from([("k".to_string(), "v".to_string())])),
            })?,
        )
        .await?;
    println!("integrationsMirror → {:?}", mirror);

    let profile = client
        .query(
            UsersUsersGetProfileArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(UsersUsersGetProfileArgs {
                user_id: user_id.clone().into(),
                with_bytes: None,
            })?,
        )
        .await?;
    println!("usersGetProfile → {:?}", profile);

    let by_email = client
        .query(
            UsersUsersGetByEmailArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(UsersUsersGetByEmailArgs {
                email: "demo@example.com".to_string(),
                include_inactive: None,
            })?,
        )
        .await?;
    println!("usersGetByEmail → {:?}", by_email);

    let _tag_update = client
        .mutation(
            ProjectsProjectsUpdateTagsArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(ProjectsProjectsUpdateTagsArgs {
                project_id: project_id.clone().into(),
                tags: vec!["rust".to_string(), "convex".to_string(), "typegen".to_string()],
            })?,
        )
        .await?;
    println!("projectsUpdateTags ok");

    let _task = client
        .mutation(
            TasksTasksCreateArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(TasksTasksCreateArgs {
                project_id: project_id.clone().into(),
                title: "From Rust client".to_string(),
                priority: TasksTasksCreateArgsPriority::Branch3(TasksTasksCreateArgsPriorityBranch3),
                assignee_user_id: Some(user_id.into()),
                payload: Some(RawValue(ConvexValue::Object(std::collections::BTreeMap::from([(
                    "source".into(),
                    "advanced example".into(),
                )])))),
                due_at: None,
            })?,
        )
        .await?;
    println!("tasksCreate ok");

    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let _alt_user = client
        .mutation(
            UsersUsersCreateArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(UsersUsersCreateArgs {
                email: format!("rust-{ts}@example.com"),
                display_name: "Rust-created".to_string(),
                role: UsersUsersCreateArgsRole::Branch2(UsersUsersCreateArgsRoleBranch2),
                metadata: Some(std::collections::BTreeMap::from([(
                    "channel".to_string(),
                    "advanced-example".to_string(),
                )])),
                score: Some(7),
                avatar_bytes: None,
            })?,
        )
        .await;
    println!("usersCreate (extra) → {:?}", _alt_user);

    Ok(())
}

fn extract_ids(summary: &FunctionResult) -> (Option<String>, Option<String>, Option<String>)
{
    let FunctionResult::Value(ConvexValue::Object(obj)) = summary else {
        return (None, None, None);
    };
    let team = obj.get("firstTeamId").and_then(string_from_convex_value);
    let project = obj.get("firstProjectId").and_then(string_from_convex_value);
    let user = obj.get("firstUserId").and_then(string_from_convex_value);
    (team, project, user)
}

fn string_from_convex_value(v: &ConvexValue) -> Option<String>
{
    match v {
        ConvexValue::String(s) => Some(s.clone()),
        _ => None,
    }
}
