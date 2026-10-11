use futures_channel::mpsc;
use futures_util::{Stream, StreamExt, stream};
use ruru_capture::new_capturer;
use ruru_config::KeychainCredentialStore;
use std::sync::Arc;

use crate::settings::Settings;
use crate::state::Core;
use crate::{Output, Sender, SettingsSnapshot, hotkey};

pub fn start() -> (Sender, impl Stream<Item = Output>, SettingsSnapshot) {
    let (sender, inputs) = mpsc::unbounded();
    let settings = Arc::new(Settings::load(Arc::new(KeychainCredentialStore::default())));
    let initial_settings = settings.snapshot();
    let core = Arc::new(Core::new(settings, Arc::from(new_capturer())));
    let hotkey_outputs = hotkey::outputs(Arc::clone(&core));
    let responses = inputs
        .map(move |input| core.handle_input(input))
        .flatten_unordered(None);

    (
        sender,
        stream::select(hotkey_outputs, responses),
        initial_settings,
    )
}
