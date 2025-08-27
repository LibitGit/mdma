use common::{
    closure, debug_log, map_err, messaging::prelude::*, web_extension_sys::runtime::port::Port,
};
use wasm_bindgen::prelude::*;

use crate::{FOREGROUND_PORT, console_error, dispatcher::Dispatcher};

#[wasm_bindgen(js_name = "handlePortConnect")]
pub async fn handle_port_connect(port: Port) {
    debug_log!("connected to:", &port.name());

    if port.name() != "foreground" {
        debug_log!("Unrecognised port:", port);
        return;
    }

    port.on_message()
        .add_listener(&closure!(|message: JsValue| async move {
            if let Err(err_code) = port_on_message(message).await {
                console_error!(err_code);
            }
        }));
    // port.on_disconnect().add_listener(&on_disconnect_factory());

    FOREGROUND_PORT.set(Some(port));
}

async fn port_on_message(message: JsValue) -> Result<(), JsValue> {
    let msg: Message = serde_wasm_bindgen::from_value(message).map_err(map_err!(from))?;
    debug_log!(@f "handlePortMessage: {msg:#?}");

    Dispatcher::dispatch_from_port(msg)
        .await
        .map_err(map_err!(from))
}

// #[derive(Serialize)]
// #[serde(rename_all = "camelCase")]
// struct Debuggee {
//     tab_id: Option<i32>,
// }

// impl Debuggee {
//     fn new(tab_id: i32) -> Self {
//         Self {
//             tab_id: Some(tab_id),
//         }
//     }
// }

// enum KeyPressType {
//     Down,
//     Up,
// }

// #[derive(Serialize)]
// struct CommandParams<'a> {
//     r#type: &'a str,
//     key: &'a str,
//     code: &'a str,
//     #[serde(rename = "windowsVirtualKeyCode")]
//     windows_virtual_key_code: usize,
// }

// #[derive(Clone, Copy)]
// enum Key {
//     W,
//     A,
//     S,
//     D,
// }

// impl TryFrom<char> for Key {
//     type Error = JsValue;
//     fn try_from(value: char) -> Result<Self, Self::Error> {
//         use Key::*;

//         match value {
//             'w' => Ok(W),
//             'a' => Ok(A),
//             's' => Ok(S),
//             'd' => Ok(D),
//             _ => Err(err_code!()),
//         }
//     }
// }

// impl CommandParams<'_> {
//     fn new(key: Key, key_press_type: KeyPressType) -> Self {
//         let (key_str, code_str, vk_code) = match key {
//             Key::W => ("w", "KeyW", 87),
//             Key::A => ("a", "KeyA", 65),
//             Key::S => ("s", "KeyS", 83),
//             Key::D => ("d", "KeyD", 68),
//         };

//         Self {
//             r#type: match key_press_type {
//                 KeyPressType::Down => "keyDown",
//                 KeyPressType::Up => "keyUp",
//             },
//             key: key_str,
//             code: code_str,
//             windows_virtual_key_code: vk_code,
//         }
//     }
// }

// fn on_message_factory() -> Function {
//     closure!(|message: JsValue, port: Port| async move {
//         debug_log!("message:", &message);

//         let task: Task = serde_wasm_bindgen::from_value(message).unwrap_js();

//         match task.type_() {
//             Tasks::UserData => on_user_data(task).await,
//             Tasks::AddonData => on_addon_data(task).await,
//             Tasks::Cookie => {
//                 let cookie_details =
// serde_wasm_bindgen::to_value(&task.cookie_details.unwrap_js())
// .unwrap_js()                     .unchecked_into();
//                 let cookie_data: Cookie =
//
// serde_wasm_bindgen::from_value(browser().cookies().get(&cookie_details).
// await)                         .unwrap_js();

//                 let cookie_response = Task::builder()
//                     .task(Tasks::Cookie)
//                     .target(Targets::Foreground)
//                     .cookie_data(cookie_data)
//                     .build()
//                     .unwrap_js()
//                     .to_value()
//                     .unwrap_js();
//                 port.post_message(&cookie_response);
//             }
//             Tasks::AttachDebugger => {
//                 let debuggee = serde_wasm_bindgen::to_value(&Debuggee::new(
//
// port.sender().unwrap_js().tab().unwrap_js().id().unwrap_js(),
// ))                 .unwrap_js();

//                 wasm_bindgen_futures::spawn_local(async move {
//                     loop {
//                         if let Err(_err) =
// browser().debugger().attach(&debuggee, "1.3").await {
// debug_log!("ERROR WHEN CONNECTING TODEBUGGER:", _err);
// };

//                         Task::delay(300_000).await;
//                     }
//                 });
//             }
//             Tasks::DetachDebugger => {
//                 let debuggee = serde_wasm_bindgen::to_value(&Debuggee::new(
//
// port.sender().unwrap_js().tab().unwrap_js().id().unwrap_js(),
// ))                 .unwrap_js();
//                 if let Err(_err) =
// browser().debugger().detach(&debuggee).await {
// debug_log!("ERROR WHEN DETACHING DEBUGGER:", _err);                 };
//             }
//             Tasks::KeyDown => {
//                 let key = task.key.unwrap_js().try_into().unwrap_js();
//                 let debuggee = serde_wasm_bindgen::to_value(&Debuggee::new(
//
// port.sender().unwrap_js().tab().unwrap_js().id().unwrap_js(),
// ))                 .unwrap_js();
//                 let command_params =
//                     serde_wasm_bindgen::to_value(&CommandParams::new(key,
// KeyPressType::Down))                         .unwrap_js();

//                 browser()
//                     .debugger()
//                     .send_command(
//                         &debuggee,
//                         s!("Input.dispatchKeyEvent"),
//                         Some(command_params.unchecked_ref()),
//                     )
//                     .await
//                     .unwrap_js();
//             }
//             Tasks::KeyUp => {
//                 let key = task.key.unwrap_js().try_into().unwrap_js();
//                 let debuggee = serde_wasm_bindgen::to_value(&Debuggee::new(
//
// port.sender().unwrap_js().tab().unwrap_js().id().unwrap_js(),
// ))                 .unwrap_js();
//                 let command_params =
//                     serde_wasm_bindgen::to_value(&CommandParams::new(key,
// KeyPressType::Up))                         .unwrap_js();

//                 browser()
//                     .debugger()
//                     .send_command(
//                         &debuggee,
//                         s!("Input.dispatchKeyEvent"),
//                         Some(command_params.unchecked_ref()),
//                     )
//                     .await
//                     .unwrap_js();
//             }
//             _ => throw_err_code!("Unrecognised task! {:?}", task),
//         }
//         //Don't add anything after this match or you WILL have to rewrite the
//         // task queue to take in
//         //(Task, VecDequeue<oneshot::Sender<()>>)
//     })
// }

// async fn on_user_data(task: Task) {
//     let Some(settings) = task.settings else {
//         Task::fetch_user_data()
//             .await
//             .unwrap_js()
//             .redirect(Targets::Foreground)
//             .execute()
//             .await
//             .unwrap_js();
//         return;
//     };

//     let Value::Array(settings_vec) = &settings else {
//         todo!("c wtedy");
//     };
//     let account_id = settings_vec[0].as_str().unwrap_js();
//     let char_id = settings_vec[1].as_str().unwrap_js();

//     let storage_settings = browser()
//         .storage()
//         .local()
//         .get(&JsValue::from_str(account_id))
//         .await;
//     let mut storage_settings =
//         serde_wasm_bindgen::from_value::<Value>(storage_settings.
// unchecked_into()).unwrap_js();

//     USER_DATA.with_borrow_mut(|data| {
//         let loaded_settings = match data.settings.take() {
//             Some(mut old_settings) => {
//                 Task::merge_json(&mut old_settings,
// storage_settings.clone());                 old_settings
//             }
//             None => storage_settings.clone(),
//         };

//         data.settings = Some(loaded_settings);
//     });

//     let character_settings = storage_settings[account_id][char_id].take();
//     debug_log!(account_id, char_id, character_settings.is_null());
//     if !character_settings.is_null() {
//         Task::builder()
//             .target(Targets::Foreground)
//             .task(Tasks::UserData)
//             .settings(character_settings)
//             .build()
//             .unwrap_js()
//             .execute()
//             .await
//             .unwrap_js();
//         return;
//     }

//     let access_token =
// Task::try_get_access_token().await.unwrap_js().unwrap_js();

//     let (sender, receiver) = oneshot::channel::<Task>();

//     PENDING_REQUESTS.with_borrow_mut(|pending| pending.insert(Task::UserData,
// sender));     debug_log!(&format!("SETTINGS SINCE NO LS CHAR FOUND:
// {settings:?}"));     //TODO: error server side in this case.
//     //NVM settings serializes to a number when trying to send it via
//     Task::builder()
//         .target(Targets::Backend)
//         .task(Tasks::UserData)
//         .access_token(access_token)
//         .settings(settings)
//         .build()
//         .unwrap_js()
//         .execute()
//         .await
//         .unwrap_js();

//     receiver
//         .await
//         .map_err(JsError::from)
//         .unwrap_js()
//         .redirect(Targets::Foreground)
//         .execute()
//         .await
//         .unwrap_js();
// }

// async fn on_addon_data(task: Task) {
//     let new_settings: Value = task.settings.unwrap_js();
//     let mut settings = match USER_DATA.with_borrow(|data|
// data.settings.clone()) {         Some(settings) => settings,
//         None => serde_wasm_bindgen::from_value(
//             browser().storage().local().get(&JsValue::NULL).await.into(),
//         )
//         .unwrap_js(),
//     };

//     Task::merge_json(&mut settings, new_settings);
//     USER_DATA.with_borrow_mut(|data| data.settings = Some(settings.clone()));

//     let user_data_task = Task::builder()
//         .task(Tasks::UserData)
//         .target(Targets::Backend)
//         .settings(settings.clone())
//         .build()
//         .unwrap_js();

//     let new_js_settings = settings
//         .as_object()
//         .unwrap_js()
//         .serialize(&Serializer::json_compatible())
//         .unwrap_js()
//         .unchecked_into();
//     debug_log!(&new_js_settings);

//     browser().storage().local().set(&new_js_settings).await;
//     user_data_task.enqueue().unwrap_js()
// }

// fn on_disconnect_factory() -> Function {
//     closure!(|port: Port| -> Result<(), JsValue> {
//         if let Some(error) = browser().runtime().last_error() {
//             debug_log!("Runtime error when disconnecting port",
// error.message());         }
//         if let Some(error) = port.error() {
//             debug_log!("Error on port disconnect:", error.message());
//         }

//         if port.name() == "foreground" {
//             FOREGROUND_PORT.set(None);
//         } else {
//             throw_err_code!("Port name not foreground");
//         }

//         Ok(())
//     })
// }

// #[derive(Serialize)]
// struct WindowCreateData<'a> {
//     focused: bool,
//     //height: i32,
//     //width: i32,
//     //left: i32,
//     //top: i32,
//     #[serde(rename = "type")]
//     _type: &'a str,
//     url: &'a str,
// }

#[wasm_bindgen(js_name = "handleMessage")]
pub async fn handle_message(message: JsValue) -> Result<(), JsValue> {
    let msg: Message = serde_wasm_bindgen::from_value(message).map_err(map_err!(from))?;
    debug_log!(@f "handleMessage: {msg:#?}");

    Dispatcher::dispatch_from_runtime(msg)
        .await
        .map_err(map_err!(from))
    // if msg.target != Target::Background || msg.kind != MessageKind::Request {
    //     return Err(err_code!());
    // }

    // match msg.task {
    //     Task::OAuth2 => {
    //         for _ in 0..=5 {
    //             let Some(tx) = AUTH_PENDING.take() else {
    //                 sleep(200).await;
    //                 continue;
    //             };

    //             return tx.send(()).map_err(|_| err_code!());
    //         }

    //         console_error!()
    //     }
    //     Task::UserData => {
    //         for _ in 0..=5 {
    //             let maybe_user_details = CONNECTION.with_borrow(|connection|
    // {                 connection.as_ref().and_then(|connection| {
    //                     connection
    //                         .user
    //                         .as_ref()
    //                         .map(|user| (user.nick.clone(), user.premium))
    //                 })
    //             });
    //             let Some((username, maybe_premium)) = maybe_user_details else
    // {                 sleep(100).await;
    //                 continue;
    //             };

    //             sleep(2000).await;
    //             return Message::builder(Task::UserData, Target::Popup,
    // MessageKind::Response)                 .username(username)
    //                 .maybe_premium(maybe_premium)
    //                 .build()
    //                 .execute()
    //                 .await;
    //         }

    //         // Not logged in.
    //         return Message::builder(Task::UserData, Target::Popup,
    // MessageKind::Response)             .build()
    //             .execute()
    //             .await;
    //     }
    //     _ => todo!(),

    //
    //     let create_data = serde_wasm_bindgen::to_value(&create_data)
    //         .map_err(map_err!())?
    //         .unchecked_into();
    //     let window = browser().windows().create(&create_data).await;
    //     let window_id = window.id().unwrap_throw();

    //     if let Err(_err) = Task::fetch_user_data_after_login(task)
    //         .await?
    //         .redirect(Targets::Popup)
    //         .execute()
    //         .await
    //     {
    //         debug_log!(_err);
    //     }

    //     loop {
    //         Task::delay(500).await;
    //         if let Err(err) = browser().windows().get(window_id).await {
    //             let err: Error = err.unchecked_into();
    //             if err.message().starts_with("No window with id:", 0) {
    //                 return Task::cancel_login();
    //                 //return Self::cancel_login("Authentication window was
    //                 // closed.")    .await
    //                 //    .unwrap_throw();
    //             }

    //             debug_log!(
    //                 "WINDOW ERROR:",
    //                 &err,
    //                 err.js_typeof(),
    //                 &format!("{:?}", err.as_string())
    //             );
    //             return Ok(());
    //         }

    //         let is_logged_in = USER_DATA
    //             .with_borrow(|data| data.username.is_some() &&
    // data.access_level.is_some());         // TODO: Check what is sent to
    // popup when user is not authorized to use         //extension.
    //         if is_logged_in
    //         //if (new_data.username.is_some() &&
    // new_data.access_level.is_some())         //    ||
    // new_data.error_msg.is_some()         {
    //             JsFuture::from(browser().windows().remove(window_id))
    //                 .await
    //                 .unwrap_throw();
    //             return Ok(());
    //         }
    //     }
    // }
    // Tasks::CancelLogin => Task::cancel_login(),
    // Tasks::OpenPopup => {
    //     Task::fetch_user_data()
    //         .await?
    //         .redirect(Targets::Popup)
    //         .execute()
    //         .await
    // }
    // _ => {
    //     debug_log!(&format!("Unrecognised task: {task:?}"));
    //     Ok(())
    // }
    // }
}
