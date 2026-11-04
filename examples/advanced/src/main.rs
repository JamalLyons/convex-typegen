//! Smoke test: seeds demo data (if needed), then calls queries, mutations, and an action
//! using every generated `*Args` struct from this example’s Convex modules.

mod convex_types;

use std::path::Path;

use convex::{ConvexClient, FunctionResult, Value as ConvexValue};
use convex_typegen_runtime::function::RawArgs;
use convex_typegen_runtime::model::RawValue;
use convex_typegen_runtime::prelude::*;
use convex_types::api::{integrations, projects, tasks, teams, users, workspace};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    dotenvy::from_filename(Path::new(env!("CARGO_MANIFEST_DIR")).join(".env.local")).ok();

    let Ok(url) = std::env::var("CONVEX_URL") else {
        eprintln!("CONVEX_URL is not set. Add it to .env.local next to this crate’s Cargo.toml.");
        eprintln!("Example paths from codegen:");
        eprintln!("  {}", integrations::integrations_mirror.path());
        eprintln!("  {}", projects::projects_list_by_team.path());
        eprintln!("  {}", tasks::tasks_search.path());
        eprintln!("  {}", workspace::workspace_summary.path());
        return Ok(());
    };

    let mut client = ConvexClient::new(&url).await?;

    let seed = client
        .mutation(
            workspace::workspace_seed_if_empty.path(),
            ConvexClient::prepare_args(RawArgs::new())?,
        )
        .await?;
    println!("workspaceSeedIfEmpty → {:?}", seed);

    let summary = client
        .query(
            workspace::workspace_summary.path(),
            ConvexClient::prepare_args(RawArgs::new())?,
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
            teams::teams_list_by_owner.path(),
            ConvexClient::prepare_args(teams::TeamsListByOwnerArgs {
                owner_user_id: user_id.clone().into(),
            })?,
        )
        .await?;
    println!("teamsListByOwner → {:?}", teams);

    let projects = client
        .query(
            projects::projects_list_by_team.path(),
            ConvexClient::prepare_args(projects::ProjectsListByTeamArgs {
                team_id: team_id.into(),
                status_filter: Some(projects::ProjectsListByTeamArgsStatusFilter::Branch2(
                    projects::ProjectsListByTeamArgsStatusFilterBranch2,
                )),
            })?,
        )
        .await?;
    println!("projectsListByTeam → {:?}", projects);

    let tasks = client
        .query(
            tasks::tasks_search.path(),
            ConvexClient::prepare_args(tasks::TasksSearchArgs {
                filter: tasks::TasksSearchArgsFilter {
                    project_id: project_id.clone().into(),
                    min_priority: Some(tasks::TasksSearchArgsFilterMinPriority::Branch2(
                        tasks::TasksSearchArgsFilterMinPriorityBranch2,
                    )),
                },
                limit: Some(10.0),
            })?,
        )
        .await?;
    println!("tasksSearch → {:?}", tasks);

    let mirror = client
        .action(
            integrations::integrations_mirror.path(),
            ConvexClient::prepare_args(integrations::IntegrationsMirrorArgs {
                body: "hello from Rust".to_string(),
                numbers: vec![1.0, 2.0, 3.0],
                flags: integrations::IntegrationsMirrorArgsFlags {
                    verbose: true,
                    trace: false,
                },
                mode: integrations::IntegrationsMirrorArgsMode::Branch1(integrations::IntegrationsMirrorArgsModeBranch1),
                extra: Some(std::collections::BTreeMap::from([("k".to_string(), "v".to_string())])),
            })?,
        )
        .await?;
    println!("integrationsMirror → {:?}", mirror);

    let profile = client
        .query(
            users::users_get_profile.path(),
            ConvexClient::prepare_args(users::UsersGetProfileArgs {
                user_id: user_id.clone().into(),
                with_bytes: None,
            })?,
        )
        .await?;
    println!("usersGetProfile → {:?}", profile);

    let by_email = client
        .query(
            users::users_get_by_email.path(),
            ConvexClient::prepare_args(users::UsersGetByEmailArgs {
                email: "demo@example.com".to_string(),
                include_inactive: None,
            })?,
        )
        .await?;
    println!("usersGetByEmail → {:?}", by_email);

    let _tag_update = client
        .mutation(
            projects::projects_update_tags.path(),
            ConvexClient::prepare_args(projects::ProjectsUpdateTagsArgs {
                project_id: project_id.clone().into(),
                tags: vec!["rust".to_string(), "convex".to_string(), "typegen".to_string()],
            })?,
        )
        .await?;
    println!("projectsUpdateTags ok");

    let _task = client
        .mutation(
            tasks::tasks_create.path(),
            ConvexClient::prepare_args(tasks::TasksCreateArgs {
                project_id: project_id.clone().into(),
                title: "From Rust client".to_string(),
                priority: tasks::TasksCreateArgsPriority::Branch3(tasks::TasksCreateArgsPriorityBranch3),
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
            users::users_create.path(),
            ConvexClient::prepare_args(users::UsersCreateArgs {
                email: format!("rust-{ts}@example.com"),
                display_name: "Rust-created".to_string(),
                role: users::UsersCreateArgsRole::Branch2(users::UsersCreateArgsRoleBranch2),
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
