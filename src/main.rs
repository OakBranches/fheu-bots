mod music;
use anyhow::Result;
use music::Handler;
use serenity::model::prelude::*;
use serenity::Client;
use songbird::driver::DecodeMode;
use songbird::{Config, SerenityInit};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let nickbot_appid: u64 = env::var("APPLICATION_ID")?.parse()?;
    let nickbot_guildid = GuildId(env::var("NICKBOT_GUILDID")?.parse()?);
    let nickbot_fcid = RoleId(env::var("NICKBOT_ROLEID")?.parse()?);
    let nickbot_token = env::var("DISCORD_TOKEN")?;
    let youtube_dl_path = env::var("YOUTUBE_DL_PATH")?;
    let songbird_config = Config::default().decode_mode(DecodeMode::Decode);

    let handler = Handler::new(nickbot_guildid, nickbot_fcid, youtube_dl_path);
    let mut client = Client::builder(nickbot_token, GatewayIntents::all())
        .event_handler(handler)
        .application_id(nickbot_appid)
        .register_songbird_from_config(songbird_config)
        .await?;

    client.start().await?;

    Ok(())
}
