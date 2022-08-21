use std::os::unix::process::parent_id;

use chrono::{Date, DateTime, NaiveDateTime, TimeZone, Utc};
use dotenv;
use openai_api;
use openai_api::api::{CompletionArgs, EngineInfo};
use roux::{Me, Reddit, Subreddit};
use tokio;
use tokio_compat_02::FutureExt;

async fn client() -> anyhow::Result<Me> {
    let username = dotenv::var("reddit_username")?;
    let password = dotenv::var("reddit_password")?;
    let id = dotenv::var("reddit_id")?;
    let secret = dotenv::var("reddit_secret")?;

    let client = Reddit::new(
        format!("linux:amelioration:0.0.1  (by /u/{username})").as_ref(),
        id.as_ref(),
        secret.as_ref(),
    )
    .username(username.as_ref())
    .password(password.as_ref())
    .login()
    .await?;
    Ok(client)
}

async fn vaushify(msg: String) -> anyhow::Result<String> {
    let base_prompt = include_str!("prompt.txt");
    let openai_token = dotenv::var("openai_token")?;
    let prompt = format!("{base_prompt}\n\nNormal: {msg}\nVerbose");
    let args = CompletionArgs::builder()
        .prompt(prompt)
        .engine("davinci")
        .temperature(1.0)
        .top_p(1.0)
        .max_tokens(256)
        .frequency_penalty(0.)
        .presence_penalty(0.)
        .best_of(20)
        .stop(vec!["\n".to_owned()])
        .build()
        .unwrap();
    let client = openai_api::Client::new(openai_token.as_ref());
    let completion = format!("{}", client.complete_prompt(args).compat().await?);
    if completion.len() > 10 && completion.starts_with(": ") && !completion.contains("\n") {
        let completion = completion[2..].to_string();
        Ok(completion)
    } else {
        Err(anyhow::anyhow!("No completion found"))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = client().await?;
    let subreddit = Subreddit::new("destiny");
    let latest_comments = subreddit.latest_comments(Some(3), Some(250)).await?;
    /*for comment in latest_comments.data.children {
        if (true) {
            let _: () = comment.data.parent_id.unwrap();
            println!(
                "+{:?}: {:?} https://reddit.com{}",
                comment.data.ups.clone().unwrap(),
                comment.data.body.clone().unwrap(),
                comment.data.permalink.clone().unwrap()
            );
        }
    }*/

    let hot = subreddit.hot(35, None).await?;
    for post in hot.data.children {
        let post_time = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp(post.data.created_utc as i64, 0),
            Utc,
        );
        let current_time = Utc::now();
        let time_since_post_creation = current_time.signed_duration_since(post_time);
        if post.data.stickied.clone() == false && time_since_post_creation.num_hours() < 4 {
            let article_id = post.data.id.clone();
            let comments = subreddit
                .article_comments(&article_id, None, Some(250))
                .await?;

            println!(
                "+{:?}: https://reddit.com{}",
                post.data.ups.clone(),
                post.data.permalink.clone()
            );
            for comment in comments.data.children {
                let comment = &comment.data;
                let body = comment.body.clone().unwrap();
                let body = html_escape::decode_html_entities(&body).to_string();
                if comment.ups.unwrap() > 10
                    && body.split(" ").count() > 8
                    && body.split(" ").count() < 20
                    && !body.contains("\n")
                {
                    println!(
                        "    +{:?}: {:?} https://reddit.com{}",
                        comment.ups.clone().unwrap(),
                        body,
                        comment.permalink.clone().unwrap()
                    );
                    let verbose = vaushify(body).await?;
                    println!("       -> {verbose}");
                    return Ok(());
                }
            }
            println!("");
        }
    }

    Ok(())
}
