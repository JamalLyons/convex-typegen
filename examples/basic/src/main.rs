mod convex_types;

use std::io::{self, Write};

use convex::{ConvexClient, Value as ConvexValue};
use convex_typegen::convex::ConvexClientExt;
use convex_types::{GetGameArgs, LossGameArgs, WinGameArgs};
use rand::Rng;

const CONVEX_URL: &str = "https://notable-orca-705.convex.cloud";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let mut client = ConvexClient::new(CONVEX_URL).await?;

    // Get current game stats using the extension trait
    let args_map = ConvexClient::prepare_args(GetGameArgs {});
    let game_stats = client.query(GetGameArgs::FUNCTION_PATH, args_map).await?;

    println!("Initial game stats response: {:?}", game_stats);

    let (wins, losses) = match game_stats {
        convex::FunctionResult::Value(value) => {
            if let ConvexValue::Object(obj) = value {
                let win_count = obj.get("win_count").map(extract_float_value).unwrap_or(0.0);
                let loss_count = obj.get("loss_count").map(extract_float_value).unwrap_or(0.0);
                (win_count as i32, loss_count as i32)
            } else {
                (0, 0)
            }
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
                let args_map = ConvexClient::prepare_args(WinGameArgs {});
                match client.mutation(WinGameArgs::FUNCTION_PATH, args_map).await {
                    Ok(result) => println!("Save win result: {:?}", result),
                    Err(e) => println!("Error saving win: {:?}", e),
                }
                break;
            }
        }

        if attempts >= MAX_ATTEMPTS {
            println!("Sorry, you've run out of attempts! The number was {}", secret_number);
            // Save loss to Convex using lossGame mutation
            let args_map = ConvexClient::prepare_args(LossGameArgs {});
            match client.mutation(LossGameArgs::FUNCTION_PATH, args_map).await {
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
    let args_map = ConvexClient::prepare_args(GetGameArgs {});
    match client.query(GetGameArgs::FUNCTION_PATH, args_map).await {
        Ok(updated_stats) => {
            if let convex::FunctionResult::Value(value) = updated_stats {
                if let ConvexValue::Object(obj) = value {
                    let win_count = obj.get("win_count").map(extract_float_value).unwrap_or(0.0);
                    let loss_count = obj.get("loss_count").map(extract_float_value).unwrap_or(0.0);
                    println!("\nUpdated record - Wins: {}, Losses: {}", win_count as i32, loss_count as i32);
                }
            }
        }
        Err(e) => println!("Error getting updated stats: {:?}", e),
    }

    Ok(())
}

fn extract_float_value(value: &ConvexValue) -> f64
{
    if let ConvexValue::Float64(f) = value {
        *f
    } else {
        0.0
    }
}
