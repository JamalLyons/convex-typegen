//! CLI demo: reads/writes game stats through generated arg structs and [`ConvexClientExt`].

mod convex_types;

use std::io::{self, Write};
use std::path::Path;

use convex::{ConvexClient, Value as ConvexValue};
use convex_typegen::prelude::*;
use convex_types::{GamesGetGameArgs, GamesLossGameArgs, GamesWinGameArgs};
use rand::Rng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::from_filename(Path::new(env!("CARGO_MANIFEST_DIR")).join(".env.local"))?;

    let mut client = ConvexClient::new(&std::env::var("CONVEX_URL")?).await?;

    // Get current game stats using the extension trait
    let game_stats = client
        .query(
            GamesGetGameArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(GamesGetGameArgs { logData: None })?,
        )
        .await?;

    println!("Initial game stats response: {:?}", game_stats);

    let (wins, losses) = match game_stats {
        convex::FunctionResult::Value(ConvexValue::Object(obj)) => {
            let win_count = obj.get("win_count").map(extract_float_value).unwrap_or(0.0);
            let loss_count = obj.get("loss_count").map(extract_float_value).unwrap_or(0.0);
            (win_count as i32, loss_count as i32)
        }
        _ => (0, 0),
    };

    println!("Welcome to the Number Guessing Game!");
    println!("Current record - Wins: {}, Losses: {}", wins, losses);
    println!("I'm thinking of a number between 1 and 100.");

    let secret_number = rand::thread_rng().gen_range(1..=100);
    let mut attempts = 0;
    const MAX_ATTEMPTS: i32 = 10;

    loop {
        print!("Enter your guess (1-100): ");
        io::stdout().flush()?;

        let mut guess = String::new();
        io::stdin().read_line(&mut guess)?;

        let guess: i32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Please enter a valid number!");
                continue;
            }
        };

        attempts += 1;

        match guess.cmp(&secret_number) {
            std::cmp::Ordering::Less => println!("Too low!"),
            std::cmp::Ordering::Greater => println!("Too high!"),
            std::cmp::Ordering::Equal => {
                println!("Congratulations! You won in {} attempts!", attempts);
                // Save win to Convex using winGame mutation
                match client
                    .mutation(
                        GamesWinGameArgs::FUNCTION_PATH,
                        ConvexClient::prepare_args(GamesWinGameArgs {})?,
                    )
                    .await
                {
                    Ok(result) => println!("Save win result: {:?}", result),
                    Err(e) => println!("Error saving win: {:?}", e),
                }
                break;
            }
        }

        if attempts >= MAX_ATTEMPTS {
            println!("Sorry, you've run out of attempts! The number was {}", secret_number);
            // Save loss to Convex using lossGame mutation
            match client
                .mutation(
                    GamesLossGameArgs::FUNCTION_PATH,
                    ConvexClient::prepare_args(GamesLossGameArgs {})?,
                )
                .await
            {
                Ok(_) => (),
                Err(e) => println!("Error saving loss: {:?}", e),
            }
            break;
        }

        println!("You have {} attempts remaining.", MAX_ATTEMPTS - attempts);
    }

    // Wait a moment for the mutation to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Get and display updated stats
    match client
        .query(
            GamesGetGameArgs::FUNCTION_PATH,
            ConvexClient::prepare_args(GamesGetGameArgs { logData: None })?,
        )
        .await
    {
        Ok(convex::FunctionResult::Value(ConvexValue::Object(obj))) => {
            let win_count = obj.get("win_count").map(extract_float_value).unwrap_or(0.0);
            let loss_count = obj.get("loss_count").map(extract_float_value).unwrap_or(0.0);
            println!("\nUpdated record - Wins: {}, Losses: {}", win_count as i32, loss_count as i32);
        }
        Ok(_) => {}
        Err(e) => println!("Error getting updated stats: {:?}", e),
    }

    Ok(())
}

fn extract_float_value(value: &ConvexValue) -> f64 {
    if let ConvexValue::Float64(f) = value {
        *f
    } else {
        0.0
    }
}
