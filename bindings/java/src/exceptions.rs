// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use jni::{JNIEnv, objects::JString};

pub(crate) trait JniExt {
    fn throw_runtime_exception(&mut self, message: impl AsRef<str>);
    fn get_str(&mut self, string: &JString, variable_name: &str) -> Option<String>;
    fn get_nullable_str(&mut self, string: &JString) -> Option<String>;
}

impl JniExt for JNIEnv<'_> {
    fn throw_runtime_exception(&mut self, message: impl AsRef<str>) {
        self.throw_new("java/lang/RuntimeException", message)
            .expect("failed to throw the exception");
    }

    fn get_str(&mut self, string: &JString, variable_name: &str) -> Option<String> {
        match self.get_string(string) {
            Ok(string) => Some(string.into()),
            Err(jni::errors::Error::NullPtr(_) | jni::errors::Error::NullDeref(_)) => {
                self.throw_new(
                    "java/lang/NullPointerException",
                    format!("`{variable_name}` is null"),
                )
                .expect("failed to throw");
                None
            }
            Err(jni::errors::Error::JavaException) => None,
            Err(error) => {
                self.throw_runtime_exception(format!(
                    "failed to get argument `{variable_name}`: {error}"
                ));
                None
            }
        }
    }

    fn get_nullable_str(&mut self, string: &JString) -> Option<String> {
        match self.get_string(string) {
            Ok(s) => Some(s.into()),
            Err(jni::errors::Error::NullPtr(_) | jni::errors::Error::NullDeref(_)) => None,
            Err(_) => None,
        }
    }
}
