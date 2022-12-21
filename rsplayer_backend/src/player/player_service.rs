use api_models::common::PlayerType;
use api_models::settings::Settings;

use super::mpd::MpdPlayerClient;
use super::{spotify::SpotifyPlayerClient, Player};
use crate::common::{MutArcConfiguration, Result};

pub struct PlayerService {
    player: Box<dyn Player + Send>,
}

impl PlayerService {
    pub fn new(config: &MutArcConfiguration) -> Result<Self> {
        let settings = config.lock().unwrap().get_settings();
        Ok(PlayerService {
            player: Self::create_player(&settings)?,
        })
    }

    pub fn get_current_player(&mut self) -> &mut Box<dyn Player + Send> {
        &mut self.player
    }

    #[allow(unreachable_patterns)]
    fn create_player(settings: &Settings) -> Result<Box<dyn Player + Send>> {
        match &settings.active_player {
            PlayerType::SPF => {
                let mut sp = SpotifyPlayerClient::new(&settings.spotify_settings)?;
                sp.start_device(&settings.alsa_settings.device_name)?;
                sp.transfer_playback_to_device()?;
                sp.play_current_track();
                Ok(Box::new(sp))
            }
            PlayerType::MPD => {
                let mut mpd = MpdPlayerClient::new(&settings.mpd_settings)?;
                mpd.ensure_mpd_server_configuration(
                    &settings.alsa_settings.device_name,
                    &settings.metadata_settings.music_directory,
                )?;
                Ok(Box::new(mpd))
            }
            _ => panic!("Unknown type"),
        }
    }
}
