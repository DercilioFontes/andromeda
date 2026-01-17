// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use andromeda_core::{Extension, ExtensionOp, HostData, OpsStorage};
use nova_vm::{
    ecmascript::{
        builtins::ArgumentsList,
        execution::{Agent, JsResult},
        types::Value,
    },
    engine::context::{Bindable, GcScope},
};

/// Test result structure
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration: u128,
}

/// Storage for test state
#[derive(Default)]
pub struct TestStorage {
    pub current_suite: Option<String>,
    pub test_results: Vec<TestResult>,
}

#[derive(Default)]
pub struct TestExt;

impl TestExt {
    pub fn new_extension() -> Extension {
        Extension {
            name: "test",
            ops: vec![
                ExtensionOp::new("__andromeda_test_describe", Self::describe, 2, true),
                ExtensionOp::new("__andromeda_test_it_passed", Self::it_passed, 1, true),
                ExtensionOp::new("__andromeda_test_it_failed", Self::it_failed, 2, true),
                ExtensionOp::new("__andromeda_test_get_results", Self::get_test_results, 0, true),
                ExtensionOp::new("__andromeda_test_reset", Self::reset_test_state, 0, true),
            ],
            storage: Some(Box::new(|storage: &mut OpsStorage| {
                storage.insert(TestStorage::default());
            })),
            files: vec![include_str!("./mod.ts")],
        }
    }

    /// Start a test suite
    fn describe<'gc>(
        agent: &mut Agent,
        _this: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let name = args[0]
            .to_string(agent, gc.reborrow())
            .unbind()?
            .as_str(agent)
            .expect("String is not valid UTF-8")
            .to_string();

        // Store the current suite name
        {
            let host_data = agent.get_host_data();
            let host_data: &HostData<crate::RuntimeMacroTask> = host_data.downcast_ref().unwrap();
            let mut storage = host_data.storage.borrow_mut();
            let test_storage: &mut TestStorage = storage.get_mut().unwrap();
            test_storage.current_suite = Some(name);
        }

        Ok(Value::Undefined)
    }

    /// Record a passed test case
    fn it_passed<'gc>(
        agent: &mut Agent,
        _this: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let name = args[0]
            .to_string(agent, gc.reborrow())
            .unbind()?
            .as_str(agent)
            .expect("String is not valid UTF-8")
            .to_string();

        // Record the passed test
        {
            let host_data = agent.get_host_data();
            let host_data: &HostData<crate::RuntimeMacroTask> = host_data.downcast_ref().unwrap();
            let mut storage = host_data.storage.borrow_mut();
            let test_storage: &mut TestStorage = storage.get_mut().unwrap();
            test_storage.test_results.push(TestResult {
                name,
                passed: true,
                error: None,
                duration: 0,
            });
        }

        Ok(Value::Undefined)
    }

    /// Record a failed test case
    fn it_failed<'gc>(
        agent: &mut Agent,
        _this: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let name = args[0]
            .to_string(agent, gc.reborrow())
            .unbind()?
            .as_str(agent)
            .expect("String is not valid UTF-8")
            .to_string();

        let error = args[1]
            .to_string(agent, gc.reborrow())
            .unbind()?
            .as_str(agent)
            .expect("String is not valid UTF-8")
            .to_string();

        // Record the failed test
        {
            let host_data = agent.get_host_data();
            let host_data: &HostData<crate::RuntimeMacroTask> = host_data.downcast_ref().unwrap();
            let mut storage = host_data.storage.borrow_mut();
            let test_storage: &mut TestStorage = storage.get_mut().unwrap();
            test_storage.test_results.push(TestResult {
                name,
                passed: false,
                error: Some(error),
                duration: 0,
            });
        }

        Ok(Value::Undefined)
    }

    /// Get test results as JSON
    fn get_test_results<'gc>(
        agent: &mut Agent,
        _this: Value,
        _args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let results = {
            let host_data = agent.get_host_data();
            let host_data: &HostData<crate::RuntimeMacroTask> = host_data.downcast_ref().unwrap();
            let storage = host_data.storage.borrow();
            let test_storage: &TestStorage = storage.get().unwrap();

            let results: Vec<_> = test_storage.test_results.iter().map(|result| {
                serde_json::json!({
                    "name": result.name,
                    "passed": result.passed,
                    "error": result.error,
                    "duration": result.duration
                })
            }).collect();

            serde_json::to_string(&results).unwrap()
        };

        Ok(Value::from_string(agent, results, gc.nogc()).unbind())
    }

    /// Reset test state
    fn reset_test_state<'gc>(
        agent: &mut Agent,
        _this: Value,
        _args: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let host_data = agent.get_host_data();
        let host_data: &HostData<crate::RuntimeMacroTask> = host_data.downcast_ref().unwrap();
        let mut storage = host_data.storage.borrow_mut();
        let test_storage: &mut TestStorage = storage.get_mut().unwrap();
        test_storage.current_suite = None;
        test_storage.test_results.clear();
        Ok(Value::Undefined)
    }
}