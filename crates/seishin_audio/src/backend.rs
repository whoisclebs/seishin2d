use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use kira::{
    sound::static_sound::StaticSoundData, AudioManager, AudioManagerSettings, DefaultBackend,
};

use crate::{AudioError, AudioSkipReason, PlaybackResult};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct AudioBackend {
    manager: AudioManager<DefaultBackend>,
    sounds: HashMap<u64, StaticSoundData>,
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct AudioBackend;

#[cfg(not(target_arch = "wasm32"))]
impl AudioBackend {
    pub(crate) fn new() -> Result<Self, String> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|error| error.to_string())?;

        Ok(Self {
            manager,
            sounds: HashMap::new(),
        })
    }

    pub(crate) fn load_sound(&mut self, id: u64, path: PathBuf) -> Result<(), AudioError> {
        let sound = StaticSoundData::from_file(&path).map_err(|error| AudioError::Decode {
            path,
            reason: error.to_string(),
        })?;

        self.sounds.insert(id, sound);
        Ok(())
    }

    pub(crate) fn play_sound(&mut self, id: u64) -> PlaybackResult {
        let Some(sound) = self.sounds.get(&id) else {
            return PlaybackResult::Skipped(AudioSkipReason::PlaybackFailed(
                "sound data was not loaded".to_string(),
            ));
        };

        match self.manager.play(sound.clone()) {
            Ok(_) => PlaybackResult::Started,
            Err(error) => {
                PlaybackResult::Skipped(AudioSkipReason::PlaybackFailed(error.to_string()))
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl AudioBackend {
    pub(crate) fn new() -> Result<Self, String> {
        Err("audio backend is not available on wasm yet".to_string())
    }

    pub(crate) fn load_sound(&mut self, _id: u64, _path: PathBuf) -> Result<(), AudioError> {
        Ok(())
    }

    pub(crate) fn play_sound(&mut self, _id: u64) -> PlaybackResult {
        PlaybackResult::Skipped(AudioSkipReason::BackendUnavailable(
            "audio backend is not available on wasm yet".to_string(),
        ))
    }
}
