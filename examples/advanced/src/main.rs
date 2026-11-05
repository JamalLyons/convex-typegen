//! Smoke test: seeds demo data (if needed), then calls queries, mutations, and an action
//! using every generated `*Args` struct from this example’s Convex modules.

#[allow(dead_code)]
mod convex_types
{
    include!(concat!(env!("OUT_DIR"), "/convex_types.rs"));
}

use std::path::Path;

use convex::{ConvexClient, Value as ConvexValue};
use convex_typegen_runtime::function::RawArgs;
use convex_typegen_runtime::model::RawValue;
use convex_typegen_runtime::TypedClient;
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

    let mut sdk = ConvexClient::new(&url).await?;
    let mut client = TypedClient::new(&mut sdk);

    let seed = client.mutation(workspace::workspace_seed_if_empty, RawArgs::new()).await?;
    println!("workspaceSeedIfEmpty → {:?}", seed);

    let summary = client.query(workspace::workspace_summary, RawArgs::new()).await?;
    println!("workspaceSummary → {:?}", summary);

    let (team_id, project_id, user_id) = extract_ids(&summary);
    let (Some(team_id), Some(project_id), Some(user_id)) = (team_id, project_id, user_id) else {
        println!("Not enough data in deployment yet (need at least one user, team, and project).");
        return Ok(());
    };

    let teams = client
        .query(
            teams::teams_list_by_owner,
            teams::ListByOwnerArgs {
                owner_user_id: user_id.clone().into(),
            },
        )
        .await?;
    println!("teamsListByOwner → {:?}", teams);

    let projects = client
        .query(
            projects::projects_list_by_team,
            projects::ListByTeamArgs {
                team_id: team_id.into(),
                status_filter: Some(projects::ListByTeamArgsStatusFilter::Branch2(
                    projects::ListByTeamArgsStatusFilterBranch2,
                )),
            },
        )
        .await?;
    println!("projectsListByTeam → {:?}", projects);

    let tasks = client
        .query(
            tasks::tasks_search,
            tasks::SearchArgs {
                filter: tasks::SearchArgsFilter {
                    project_id: project_id.clone().into(),
                    min_priority: Some(tasks::SearchArgsFilterMinPriority::Branch2(
                        tasks::SearchArgsFilterMinPriorityBranch2,
                    )),
                },
                limit: Some(10.0),
            },
        )
        .await?;
    println!("tasksSearch → {:?}", tasks);

    let mirror = client
        .action(
            integrations::integrations_mirror,
            integrations::MirrorArgs {
                body: "hello from Rust".to_string(),
                numbers: vec![1.0, 2.0, 3.0],
                flags: integrations::MirrorArgsFlags {
                    verbose: true,
                    trace: false,
                },
                mode: integrations::MirrorArgsMode::Branch1(integrations::MirrorArgsModeBranch1),
                extra: Some(std::collections::BTreeMap::from([("k".to_string(), "v".to_string())])),
            },
        )
        .await?;
    println!("integrationsMirror → {:?}", mirror);

    let profile = client
        .query(
            users::users_get_profile,
            users::GetProfileArgs {
                user_id: user_id.clone().into(),
                with_bytes: None,
            },
        )
        .await?;
    println!("usersGetProfile → {:?}", profile);

    let by_email = client
        .query(
            users::users_get_by_email,
            users::GetByEmailArgs {
                email: "demo@example.com".to_string(),
                include_inactive: None,
            },
        )
        .await?;
    println!("usersGetByEmail → {:?}", by_email);

    let _tag_update = client
        .mutation(
            projects::projects_update_tags,
            projects::UpdateTagsArgs {
                project_id: project_id.clone().into(),
                tags: vec!["rust".to_string(), "convex".to_string(), "typegen".to_string()],
            },
        )
        .await?;
    println!("projectsUpdateTags ok");

    let _task = client
        .mutation(
            tasks::tasks_create,
            tasks::CreateArgs {
                project_id: project_id.clone().into(),
                title: "From Rust client".to_string(),
                priority: tasks::CreateArgsPriority::Branch3(tasks::CreateArgsPriorityBranch3),
                assignee_user_id: Some(user_id.into()),
                payload: Some(RawValue(ConvexValue::Object(std::collections::BTreeMap::from([(
                    "source".into(),
                    "advanced example".into(),
                )])))),
                due_at: None,
            },
        )
        .await?;
    println!("tasksCreate ok");

    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let _alt_user = client
        .mutation(
            users::users_create,
            users::CreateArgs {
                email: format!("rust-{ts}@example.com"),
                display_name: "Rust-created".to_string(),
                role: users::CreateArgsRole::Branch2(users::CreateArgsRoleBranch2),
                metadata: Some(std::collections::BTreeMap::from([(
                    "channel".to_string(),
                    "advanced-example".to_string(),
                )])),
                score: Some(7),
                avatar_bytes: None,
            },
        )
        .await;
    println!("usersCreate (extra) → {:?}", _alt_user);

    Ok(())
}

fn extract_ids(summary: &RawValue) -> (Option<String>, Option<String>, Option<String>)
{
    let ConvexValue::Object(obj) = &summary.0 else {
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
