
use std::hash::{Hash};
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use std::time::Duration;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{not_line_ending, space1};
use nom::{IResult, Parser};
use rand::Rng;
use tokio::{
    io::{self, AsyncBufReadExt, BufReader},
    signal,
};
use messages_p2p::p2p::handlers::SimpleClientHandler;
use messages_p2p::p2p::node::{ChatCommand, ClientNode, run_node};

pub fn generate_rand_msg() -> String {
    let mut rng = rand::rng();
    let random_number: u32 = rng.random_range(0..10000);
    format!("Random message: {}", random_number)
}





#[tokio::main]
pub async fn main() -> anyhow::Result<()> {

    /* */
    let node = run_node()?;
    let tx = node.lock().await.command_sender();



    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("Connected to chat-room. Type  /quit, /publish <channel> <msg>, or /join <channel> to join a channel.");

    loop {
        tokio::select! {
        // Read input from stdin
        maybe_line = lines.next_line() => {
            match maybe_line {
                Ok(Some(line)) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with("/") {
                            let is_exit: bool = handle_command(trimmed, &tx).await;
                            if is_exit {
                                println!("Node is leaving the network...");
                                break;
                            }
                    } else {
                        println!("Unknown input. Use commands like /publish <channel> <msg>, /quit, /join <channel>.");
                    }
                }
                Ok(None) => {
                    println!("Stdin closed.");
                    break;
                }
                Err(e) => {
                    eprintln!("Error reading input: {e}");
                    break;
                }
            }
        }

        // Optionally: Ctrl+C handling (graceful shutdown)
        _ = signal::ctrl_c() => {
            println!("\nShutting down...");
            break;
        }
    }
    }

    Ok(())
}

async fn handle_command(command: &str, sender: &mpsc::Sender<ChatCommand>) -> bool{

    match parse_commands(command) {
        Ok((_, ChatCommand::Quit)) => {
            println!("Exiting...");
            return true;
        }
        Ok((_, ChatCommand::Subscribe(channel))) => {
            println!("Subscribing to channel: {}", channel);
            let _ = sender.send(ChatCommand::Subscribe(channel)).await;
        }
        Ok((_, ChatCommand::Publish(channel, msg))) => {
            println!("Publishing to channel: {} with message: {}", channel, String::from_utf8_lossy(&msg));
            let _ = sender.send(ChatCommand::Publish(channel, msg)).await;
        }
        Err(e) => {
            eprintln!("Failed to parse command: {e}");
        }
    }
    false
}


fn parse_commands(input: &str) -> IResult<&str, ChatCommand> {
    let (input, character) = nom::bytes::complete::take(1usize)(input)?; //
    if character != "/" {
        return Err(nom::Err::Error(nom::error::Error {
            input: input,
            code: nom::error::ErrorKind::Tag,
        }));
    }
    alt((parse_quit, parse_subscribe, parse_publish)).parse(input)
}

fn parse_quit(input: &str) -> IResult<&str, ChatCommand> {
    let (input, _) = tag("quit")(input)?;
    Ok((input, ChatCommand::Quit))
}

fn parse_subscribe(input: &str) -> IResult<&str, ChatCommand> {
    let (input, _) = tag("join")(input)?;
    let (input, _) = space1(input)?;
    Ok((input, ChatCommand::Subscribe(input.to_string())))
}

fn parse_publish(input: &str) -> IResult<&str, ChatCommand> {
    let (input, _) = tag("publish")(input)?;
    let (input, _) = space1(input)?;
    let (input, topic) = take_while1(|c: char| !c.is_whitespace())(input)?;
    let (input, _) = space1(input)?;
    let (input, msg) = not_line_ending(input)?;

    Ok((input, ChatCommand::Publish(topic.to_string(), msg.to_string().into_bytes())))
}
