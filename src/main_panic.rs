use std::any::Any;

pub(crate) fn install_broken_pipe_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if panic_payload_is_broken_pipe(info.payload()) {
            return;
        }
        default_hook(info);
    }));
}

pub(crate) fn panic_payload_is_broken_pipe(payload: &(dyn Any + Send)) -> bool {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        msg.contains("Broken pipe")
    } else if let Some(msg) = payload.downcast_ref::<String>() {
        msg.contains("Broken pipe")
    } else {
        false
    }
}
