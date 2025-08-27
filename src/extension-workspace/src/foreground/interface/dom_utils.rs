use common::err_code;
use futures_signals::signal_vec::{MutableVec, VecDiff};
use js_sys::Array;
use wasm_bindgen::JsValue;
use web_sys::{DomTokenList, HtmlElement};

use crate::console_error;
use crate::utils::JsResult;

pub(crate) trait ClassListSignal {
    fn replace(&self, values: &[&'static str]) -> JsResult<()>;
    fn insert_at(&self, value: &'static str) -> JsResult<()>;
    fn update_at(&self, index: u32, value: &'static str) -> JsResult<()>;
    fn remove_at(&self, index: u32) -> JsResult<()>;
    fn push(&self, value: &'static str) -> JsResult<()>;
    fn pop(&self) -> JsResult<()>;
    fn clear(&self) -> JsResult<()>;
}

impl ClassListSignal for DomTokenList {
    fn replace(&self, values: &[&'static str]) -> JsResult<()> {
        self.add(
            &values
                .iter()
                .map(|&value| JsValue::from_str(value))
                .collect(),
        )
    }

    //TODO: Perform insert_at not push_back..
    fn insert_at(&self, value: &'static str) -> JsResult<()> {
        self.add_1(value)
    }

    fn update_at(&self, index: u32, value: &'static str) -> JsResult<()> {
        let old_token = self.get(index).ok_or_else(|| err_code!())?;

        self.replace(&old_token, value).map(|_| ())
    }

    fn remove_at(&self, index: u32) -> JsResult<()> {
        let token = self.get(index).ok_or_else(|| err_code!())?;

        self.remove_1(&token)
    }

    fn push(&self, value: &'static str) -> JsResult<()> {
        self.add_1(value)
    }

    fn pop(&self) -> JsResult<()> {
        let token = self.get(self.length() - 1).ok_or_else(|| err_code!())?;

        self.remove_1(&token)
    }

    fn clear(&self) -> JsResult<()> {
        self.remove(&Array::from(self))
    }
}

pub(crate) trait ClassListVec<'a> {
    //fn add(&mut self, class: &'a str) -> Result<()>;
    //fn remove(&mut self, class: &'a str);
    fn update_on_change(&self, element: &HtmlElement)
    where
        Self: 'static;
}

impl<'a> ClassListVec<'a> for MutableVec<&'a str> {
    //fn add(&mut self, class: &'a str) -> Result<()> {
    //    let mut self_lock = self.lock_mut();
    //    if self_lock.contains(&class) {
    //        return Ok(());
    //    }
    //    if class.is_empty() {
    //        //TODO: Change to SyntaxError.
    //        return Err(error::std::obf_get!("class"));
    //    }
    //    if class.chars().any(|c| c.is_whitespace()) {
    //        //TODO: Change to InvalidCharacterError.
    //        return Err(error::std::obf_get!("class"));
    //    }
    //
    //    self_lock.push(class);
    //
    //    Ok(())
    //}

    //fn remove(&mut self, class: &'a str) {
    //    let mut self_lock = self.lock_mut();
    //    self_lock.retain(|c| c != &class);
    //}

    fn update_on_change(&self, element: &HtmlElement)
    where
        Self: 'static,
    {
        use futures_signals::signal_vec::SignalVecExt;

        let class_list = element.class_list();
        let class_list_future = self.signal_vec().for_each(move |change| {
            let change_result = match change {
                VecDiff::Replace { values } => ClassListSignal::replace(&class_list, &values),
                VecDiff::InsertAt { value, .. } => class_list.insert_at(value),
                VecDiff::UpdateAt { index, value } => class_list.update_at(index as u32, value),
                VecDiff::RemoveAt { index } => class_list.remove_at(index as u32),
                VecDiff::Push { value } => class_list.push(value),
                VecDiff::Pop {} => class_list.pop(),
                VecDiff::Clear {} => class_list.clear(),
                //TODO: Method for handling this case (for whatever reason).
                VecDiff::Move { .. } => Ok(()),
            };

            if let Err(err) = change_result {
                console_error!(err);
            }

            async {}
        });

        wasm_bindgen_futures::spawn_local(async move {
            class_list_future.await;
        });
    }
}
