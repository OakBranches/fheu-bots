use std::{fmt::format, result};

use anyhow::{anyhow, Result};

use serenity::{
    model::{
        gateway::Ready,
        interactions::{
            application_command::{
                ApplicationCommandInteraction as ACInt,
                ApplicationCommandInteractionDataOption as Option,
                ApplicationCommandInteractionDataOptionValue as OptionValue,
                ApplicationCommandOptionType as OptionType,
            },
            Interaction,
        },
        prelude::*,
    },
    prelude::*,
};

use youtube_dl::SearchOptions;
use youtube_dl::YoutubeDl;

macro_rules! how {
    ($e: expr, $($err: tt)*) => {
        match $e {
            Some(v) => Ok(v),
            None => Err(anyhow!($($err)*)),
        }
    }
}

pub struct Handler {
    guild_id: GuildId,
    fanclub_role: RoleId,
    youtube_dl_path: String,
}

impl Handler {
    pub fn new(
        guild_id: GuildId,
        fanclub_role: RoleId,
        youtube_dl_path: String,
    ) -> Self {
        Self {
            guild_id,
            fanclub_role,
            youtube_dl_path,
        }
    }

    async fn join(&self, ctx: &Context, int: &ACInt) -> Result<bool> {
        //get voice channel
        let channels = ctx
            .http
            .get_channels(int.guild_id.unwrap().as_u64().clone())
            .await?;

        let user_id = int.member.as_ref().unwrap().user.id.as_u64().clone();

        let mut voice_channel = None;

        for channel in channels.iter() {
            if channel.kind == ChannelType::Voice {
                let members = channel.members(ctx.cache.clone()).await;
                let members = members?;
                for member in members.iter() {
                    if member.user.id.as_u64().clone() == user_id {
                        voice_channel = Some(channel.clone());
                        break;
                    }
                }
            }
        }
        dbg!(&voice_channel);

        if voice_channel.is_none() {
            self.update_response(ctx, int, "Você não está em um canal de voz")
                .await?;
            return Ok(false);
        }

        let voice_channel =
            how!(voice_channel, "Você não está em um canal de voz")?;

        let manager = songbird::get(ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        let _handler = manager.join(self.guild_id, voice_channel.id).await;

        Ok(true)
    }

    async fn search_song(
        &self,
        ctx: &Context,
        int: &ACInt,
        nome: String,
    ) -> Result<()> {
        let search = SearchOptions::youtube(nome);
        self.response(ctx, int, "Pensando se eu te respondo..")
            .await?;
        let joinned = self.join(ctx, int).await?;
        if !joinned {
            return Ok(());
        }
        let yt_path = self.youtube_dl_path.clone();

        let founded_video = tokio::spawn(async move {
            let output = YoutubeDl::search_for(&search)
                .youtube_dl_path(yt_path)
                .socket_timeout("15")
                .run();
            let video = match output.unwrap() {
                youtube_dl::YoutubeDlOutput::SingleVideo(v) => v,
                youtube_dl::YoutubeDlOutput::Playlist(vl) => {
                    Box::new(vl.entries.unwrap().into_iter().next().unwrap())
                }
            };
            video
        });
        let resposta =
            format!("Tocando: {}", founded_video.await.unwrap().title);
        self.update_response(ctx, int, &resposta).await?;

        Ok(())
    }

    async fn cmd_play(&self, ctx: &Context, int: &ACInt) -> Result<()> {
        let song_name = match int.data.options.get(0) {
            Some(Option {
                resolved: Some(OptionValue::String(song_name)),
                ..
            }) => song_name,
            Some(_) => return Err(anyhow!("Invalid argument #1")),
            None => return Err(anyhow!("Command requires argument #1")),
        };

        self.search_song(ctx, int, song_name.clone()).await
    }

    async fn handle_fallible(&self, ctx: &Context, int: &ACInt) {
        let result = match int.data.name.as_str() {
            "play" => self.cmd_play(ctx, int).await,
            _ => unreachable!(),
        };

        if let Err(err) = result {
            self.response(ctx, int, &format!("Error: {}", err))
                .await
                .ok();
        }
    }

    async fn response(
        &self,
        ctx: &Context,
        int: &ACInt,
        response: &str,
    ) -> Result<()> {
        int.create_interaction_response(&ctx.http, |res| {
            res.interaction_response_data(|msg| msg.content(response))
        })
        .await
        .map_err(|e| e.into())
    }

    async fn update_response(
        &self,
        ctx: &Context,
        int: &ACInt,
        response: &str,
    ) -> Result<serenity::model::channel::Message> {
        int.edit_original_interaction_response(&ctx.http, |res| {
            res.content(response)
        })
        .await
        .map_err(|e| e.into())
    }
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, int: Interaction) {
        if let Interaction::ApplicationCommand(int) = int {
            self.handle_fallible(&ctx, &int).await;
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!(
            "{} entrou usando o link de convite do grupo!",
            ready.user.name
        );

        self.guild_id
            .create_application_command(&ctx.http, |cmd| {
                cmd.name("play").description("Toca-disco").create_option(
                    |opt| {
                        opt.name("a_braba")
                            .description("Ativa o toca-disco")
                            .kind(OptionType::String)
                            .required(true)
                    },
                )
            })
            .await
            .unwrap();
    }
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: Result<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}
