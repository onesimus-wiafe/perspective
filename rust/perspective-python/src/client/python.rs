// ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ ██████ ██████ ██████       █      █      █      █      █ █▄  ▀███ █       ┃
// ┃ ▄▄▄▄▄█ █▄▄▄▄▄ ▄▄▄▄▄█  ▀▀▀▀▀█▀▀▀▀▀ █ ▀▀▀▀▀█ ████████▌▐███ ███▄  ▀█ █ ▀▀▀▀▀ ┃
// ┃ █▀▀▀▀▀ █▀▀▀▀▀ █▀██▀▀ ▄▄▄▄▄ █ ▄▄▄▄▄█ ▄▄▄▄▄█ ████████▌▐███ █████▄   █ ▄▄▄▄▄ ┃
// ┃ █      ██████ █  ▀█▄       █ ██████      █      ███▌▐███ ███████▄ █       ┃
// ┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
// ┃ Copyright (c) 2017, the Perspective Authors.                              ┃
// ┃ ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌ ┃
// ┃ This file is part of the Perspective library, distributed under the terms ┃
// ┃ of the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). ┃
// ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

use std::any::Any;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use async_lock::RwLock;
use futures::lock::Mutex;
use perspective_client::proto::ViewOnUpdateResp;
use perspective_client::{
    assert_table_api, assert_view_api, clone, Client, ClientError, IntoBoxFnPinBoxFut,
    OnUpdateMode, OnUpdateOptions, Table, TableData, TableInitOptions, UpdateData, UpdateOptions,
    View, ViewWindow,
};
use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyFunction, PyList, PyString};
use pythonize::depythonize_bound;

#[derive(Clone)]
pub struct PyClient {
    client: Client,
    loop_cb: Arc<RwLock<Option<Py<PyFunction>>>>,
}

#[pyclass]
pub struct BoxedPyFn(Box<dyn Fn() + 'static + Send + Sync>);

impl BoxedPyFn {
    pub fn new<F: Fn() + 'static + Send + Sync>(f: F) -> Self {
        BoxedPyFn(Box::new(f))
    }
}

#[pymethods]
impl BoxedPyFn {
    fn __call__(&self) {
        (self.0)();
    }
}

// async fn process_message(
//     server: Server,
//     client: Client,
//     loop_cb: Arc<RwLock<Py<PyFunction>>>,
//     client_id: u32,
//     msg: Vec<u8>,
// ) {
//     let batch = server.handle_request(client_id, &msg);
//     for (_client_id, response) in batch {
//         client.handle_response(&response).await.unwrap()
//     }
//     let fun = BoxedPyFn::new(move || {
//         for (_client_id, response) in server.poll() {
//             client.handle_response(&response).block_on().unwrap()
//         }
//     });
//     Python::with_gil(move |py| {
//         loop_cb
//             .read()
//             .expect("lock poisoned")
//             .call1(py, (fun.into_py(py),))
//     })
//     // TODO: Make this entire function fallible
//     .expect("Unhandled exception in loop callback");
//     // )
// }

#[extend::ext]
pub impl<T> Result<T, ClientError> {
    fn into_pyerr(self) -> PyResult<T> {
        match self {
            Ok(x) => Ok(x),
            Err(x) => Err(PerspectivePyError::new_err(format!("{}", x))),
        }
    }
}

create_exception!(
    perspective,
    PerspectivePyError,
    pyo3::exceptions::PyException
);

#[pyfunction]
fn default_serializer(obj: &Bound<PyAny>) -> PyResult<String> {
    if let Ok(dt) = obj.downcast::<pyo3::types::PyDateTime>() {
        Ok(dt.str()?.to_string())
    } else if let Ok(d) = obj.downcast::<pyo3::types::PyDate>() {
        Ok(d.str()?.to_string())
    } else if let Ok(d) = obj.downcast::<pyo3::types::PyTime>() {
        Ok(d.str()?.to_string())
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "Object type not serializable",
        ))
    }
}

#[extend::ext]
impl UpdateData {
    fn from_py_partial(py: Python<'_>, input: &Py<PyAny>) -> Result<Option<UpdateData>, PyErr> {
        if let Ok(pybytes) = input.downcast_bound::<PyBytes>(py) {
            Ok(Some(UpdateData::Arrow(pybytes.as_bytes().to_vec().into())))
        } else if let Ok(pystring) = input.downcast_bound::<PyString>(py) {
            Ok(Some(UpdateData::Csv(pystring.extract::<String>()?)))
        } else if let Ok(pylist) = input.downcast_bound::<PyList>(py) {
            let json_module = PyModule::import_bound(py, "json")?;
            let kwargs = PyDict::new_bound(py);
            kwargs.set_item("default", wrap_pyfunction_bound!(default_serializer, py)?)?;
            let string = json_module.call_method("dumps", (pylist,), Some(&kwargs))?;
            Ok(Some(UpdateData::JsonRows(string.extract::<String>()?)))
        } else if let Ok(pydict) = input.downcast_bound::<PyDict>(py) {
            if pydict.keys().is_empty() {
                return Err(PyValueError::new_err("Cannot infer type of empty dict"));
            }

            let first_key = pydict.keys().get_item(0)?;
            let first_item = pydict
                .get_item(first_key)?
                .ok_or_else(|| PyValueError::new_err("Bad Input"))?;

            if first_item.downcast::<PyList>().is_ok() {
                let json_module = PyModule::import_bound(py, "json")?;
                let kwargs = PyDict::new_bound(py);
                kwargs.set_item("default", wrap_pyfunction_bound!(default_serializer, py)?)?;
                let string = json_module.call_method("dumps", (pydict,), Some(&kwargs))?;
                Ok(Some(UpdateData::JsonColumns(string.extract::<String>()?)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn from_py(py: Python<'_>, input: &Py<PyAny>) -> Result<UpdateData, PyErr> {
        if let Some(x) = Self::from_py_partial(py, input)? {
            Ok(x)
        } else {
            Err(PyValueError::new_err(format!(
                "Unknown input type {:?}",
                input.type_id()
            )))
        }
    }
}

#[extend::ext]
impl TableData {
    fn from_py(py: Python<'_>, input: Py<PyAny>) -> Result<TableData, PyErr> {
        if let Some(update) = UpdateData::from_py_partial(py, &input)? {
            Ok(TableData::Update(update))
        } else if let Ok(pylist) = input.downcast_bound::<PyList>(py) {
            let json_module = PyModule::import_bound(py, "json")?;
            let kwargs = PyDict::new_bound(py);
            kwargs.set_item("default", wrap_pyfunction_bound!(default_serializer, py)?)?;
            let string = json_module.call_method("dumps", (pylist,), Some(&kwargs))?;
            Ok(TableData::Update(UpdateData::JsonRows(
                string.extract::<String>()?,
            )))
        } else if let Ok(pydict) = input.downcast_bound::<PyDict>(py) {
            let first_key = pydict.keys().get_item(0)?;
            let first_item = pydict
                .get_item(first_key)?
                .ok_or_else(|| PyValueError::new_err("Bad Input"))?;
            if first_item.downcast::<PyList>().is_ok() {
                let json_module = PyModule::import_bound(py, "json")?;
                let kwargs = PyDict::new_bound(py);
                kwargs.set_item("default", wrap_pyfunction_bound!(default_serializer, py)?)?;
                let string = json_module.call_method("dumps", (pydict,), Some(&kwargs))?;
                Ok(TableData::Update(UpdateData::JsonColumns(
                    string.extract::<String>()?,
                )))
            } else {
                let mut schema = vec![];
                for (key, val) in pydict.into_iter() {
                    schema.push((
                        key.extract::<String>()?,
                        val.extract::<String>()?.as_str().try_into().into_pyerr()?,
                    ));
                }

                Ok(TableData::Schema(schema))
            }
        } else {
            Err(PyValueError::new_err(format!(
                "Unknown input type {:?}",
                input.type_id()
            )))
        }
    }
}

const PSP_CALLBACK_ID: &str = "__PSP_CALLBACK_ID__";

impl PyClient {
    pub fn new(handle_request: Py<PyFunction>) -> Self {
        let client = Client::new({
            move |_client, msg| {
                let msg = msg.to_vec();
                clone!(handle_request);
                async move {
                    // TODO this is not great error handling, would be nice if
                    // we could tunnel errors back to the caller which sent the
                    // message.
                    Python::with_gil(move |py| {
                        handle_request.call1(py, (PyBytes::new_bound(py, &msg),))
                    })
                    .map_err(|_| ClientError::Internal("Internal Error".to_string()))?;
                    Ok(())
                }
            }
        });

        PyClient {
            client,
            loop_cb: Arc::default(),
        }
    }

    pub async fn handle_response(&self, bytes: Py<PyBytes>) -> PyResult<()> {
        self.client
            .handle_response(Python::with_gil(|py| bytes.as_bytes(py)))
            .await
            .into_pyerr()
    }

    // // TODO
    // pub async fn init(&self) -> PyResult<()> {
    //     self.client.init().await.into_pyerr()
    // }

    pub async fn table(
        &self,
        input: Py<PyAny>,
        limit: Option<u32>,
        index: Option<Py<PyString>>,
        name: Option<Py<PyString>>,
    ) -> PyResult<PyTable> {
        let client = self.client.clone();
        let py_client = self.clone();
        let table = Python::with_gil(|py| {
            let mut options = TableInitOptions {
                name: name.map(|x| x.extract::<String>(py)).transpose()?,
                ..TableInitOptions::default()
            };

            match (limit, index) {
                (None, None) => {},
                (None, Some(index)) => {
                    options.index = Some(index.extract::<String>(py)?);
                },
                (Some(limit), None) => options.limit = Some(limit),
                (Some(_), Some(_)) => {
                    Err(PyValueError::new_err("Cannot set both `limit` and `index`"))?
                },
            };

            let table_data = TableData::from_py(py, input)?;
            let table = client.table(table_data, options);
            Ok::<_, PyErr>(table)
        })?;

        let table = table.await.into_pyerr()?;
        Ok(PyTable {
            table: Arc::new(Mutex::new(table)),
            client: py_client,
        })
    }

    pub async fn open_table(&self, name: String) -> PyResult<PyTable> {
        let client = self.client.clone();
        let py_client = self.clone();
        let table = client.open_table(name).await.into_pyerr()?;
        Ok(PyTable {
            table: Arc::new(Mutex::new(table)),
            client: py_client,
        })
    }

    pub async fn get_hosted_table_names(&self) -> PyResult<Vec<String>> {
        self.client.get_hosted_table_names().await.into_pyerr()
    }

    pub async fn set_loop_cb(&self, loop_cb: Py<PyFunction>) -> PyResult<()> {
        *self.loop_cb.write().await = Some(loop_cb);
        Ok(())
    }
}

#[derive(Clone)]
pub struct PyTable {
    table: Arc<Mutex<Table>>,
    client: PyClient,
}

assert_table_api!(PyTable);

impl PyTable {
    pub async fn get_index(&self) -> Option<String> {
        self.table.lock().await.get_index()
    }

    pub async fn get_limit(&self) -> Option<u32> {
        self.table.lock().await.get_limit()
    }

    pub async fn size(&self) -> PyResult<usize> {
        self.table.lock().await.size().await.into_pyerr()
    }

    pub async fn columns(&self) -> PyResult<Vec<String>> {
        self.table.lock().await.columns().await.into_pyerr()
    }

    pub async fn clear(&self) -> PyResult<()> {
        self.table.lock().await.clear().await.into_pyerr()
    }

    pub async fn delete(&self) -> PyResult<()> {
        self.table.lock().await.delete().await.into_pyerr()
    }

    pub async fn make_port(&self) -> PyResult<i32> {
        self.table.lock().await.make_port().await.into_pyerr()
    }

    pub async fn on_delete(&self, callback_py: Py<PyFunction>) -> PyResult<u32> {
        let loop_cb = self.client.loop_cb.read().await.clone();
        let callback = {
            let callback_py = callback_py.clone();
            Box::new(move || {
                Python::with_gil(|py| {
                    if let Some(loop_cb) = &loop_cb {
                        loop_cb.call1(py, (&callback_py,))?;
                    } else {
                        callback_py.call0(py)?;
                    }
                    Ok(()) as PyResult<()>
                })
                .expect("`on_delete()` callback failed");
            })
        };

        let callback_id = self
            .table
            .lock()
            .await
            .on_delete(callback)
            .await
            .into_pyerr()?;

        Python::with_gil(move |py| callback_py.setattr(py, PSP_CALLBACK_ID, callback_id))?;
        Ok(callback_id)
    }

    pub async fn remove_delete(&self, callback: Py<PyFunction>) -> PyResult<()> {
        let callback_id =
            Python::with_gil(|py| callback.getattr(py, PSP_CALLBACK_ID)?.extract(py))?;
        self.table
            .lock()
            .await
            .remove_delete(callback_id)
            .await
            .into_pyerr()
    }

    pub async fn remove(&self, input: Py<PyAny>) -> PyResult<()> {
        let table = self.table.lock().await;
        let table_data = Python::with_gil(|py| UpdateData::from_py(py, &input))?;
        table.remove(table_data).await.into_pyerr()
    }

    pub async fn replace(&self, input: Py<PyAny>) -> PyResult<()> {
        let table = self.table.lock().await;
        let table_data = Python::with_gil(|py| UpdateData::from_py(py, &input))?;
        table.replace(table_data).await.into_pyerr()
    }

    pub async fn update(
        &self,
        input: Py<PyAny>,
        format: Option<String>,
        port_id: Option<u32>,
    ) -> PyResult<()> {
        let table = self.table.lock().await;
        let table_data = Python::with_gil(|py| UpdateData::from_py(py, &input))?;
        let options = UpdateOptions { format, port_id };
        table.update(table_data, options).await.into_pyerr()?;
        Ok(())
    }

    pub async fn validate_expressions(&self, expressions: Py<PyAny>) -> PyResult<Py<PyAny>> {
        let expressions =
            Python::with_gil(|py| depythonize_bound(expressions.into_bound(py).into_any()))?;
        let records = self
            .table
            .lock()
            .await
            .validate_expressions(expressions)
            .await
            .into_pyerr()?;

        Python::with_gil(|py| Ok(pythonize::pythonize(py, &records)?))
    }

    pub async fn schema(&self) -> PyResult<HashMap<String, String>> {
        let schema = self.table.lock().await.schema().await.into_pyerr()?;
        Ok(schema
            .into_iter()
            .map(|(x, y)| (x, format!("{}", y)))
            .collect())
    }

    pub async fn view(&self, kwargs: Option<Py<PyDict>>) -> PyResult<PyView> {
        let config = kwargs
            .map(|config| {
                Python::with_gil(|py| depythonize_bound(config.into_bound(py).into_any()))
            })
            .transpose()?;

        let view = self.table.lock().await.view(config).await.into_pyerr()?;
        Ok(PyView {
            view: Arc::new(Mutex::new(view)),
            client: self.client.clone(),
        })
    }
}

#[derive(Clone)]
pub struct PyView {
    view: Arc<Mutex<View>>,
    client: PyClient,
}

assert_view_api!(PyView);

impl PyView {
    pub async fn column_paths(&self) -> PyResult<Vec<String>> {
        self.view.lock().await.column_paths().await.into_pyerr()
    }

    pub async fn delete(&self) -> PyResult<()> {
        self.view.lock().await.delete().await.into_pyerr()
    }

    pub async fn dimensions(&self) -> PyResult<Py<PyAny>> {
        let dim = self.view.lock().await.dimensions().await.into_pyerr()?;
        Ok(Python::with_gil(|py| pythonize::pythonize(py, &dim))?)
    }

    pub async fn expression_schema(&self) -> PyResult<HashMap<String, String>> {
        Ok(self
            .view
            .lock()
            .await
            .expression_schema()
            .await
            .into_pyerr()?
            .into_iter()
            .map(|(k, v)| (k, format!("{}", v)))
            .collect())
    }

    pub async fn get_config(&self) -> PyResult<Py<PyAny>> {
        let config = self.view.lock().await.get_config().await.into_pyerr()?;
        Ok(Python::with_gil(|py| pythonize::pythonize(py, &config))?)
    }

    pub async fn get_min_max(&self, name: String) -> PyResult<(String, String)> {
        self.view.lock().await.get_min_max(name).await.into_pyerr()
    }

    pub async fn num_rows(&self) -> PyResult<u32> {
        self.view.lock().await.num_rows().await.into_pyerr()
    }

    pub async fn schema(&self) -> PyResult<HashMap<String, String>> {
        Ok(self
            .view
            .lock()
            .await
            .schema()
            .await
            .into_pyerr()?
            .into_iter()
            .map(|(k, v)| (k, format!("{}", v)))
            .collect())
    }

    pub async fn on_delete(&self, callback_py: Py<PyFunction>) -> PyResult<u32> {
        let callback = {
            let callback_py = callback_py.clone();
            let loop_cb = self.client.loop_cb.read().await.clone();
            Box::new(move || {
                let loop_cb = loop_cb.clone();
                Python::with_gil(|py| {
                    if let Some(loop_cb) = &loop_cb {
                        loop_cb.call1(py, (&callback_py,))?;
                    } else {
                        callback_py.call0(py)?;
                    }

                    Ok(()) as PyResult<()>
                })
                .expect("`on_delete()` callback failed");
            })
        };

        let callback_id = self
            .view
            .lock()
            .await
            .on_delete(callback)
            .await
            .into_pyerr()?;
        Python::with_gil(move |py| callback_py.setattr(py, PSP_CALLBACK_ID, callback_id))?;
        Ok(callback_id)
    }

    pub async fn remove_delete(&self, callback: Py<PyFunction>) -> PyResult<()> {
        let callback_id =
            Python::with_gil(|py| callback.getattr(py, PSP_CALLBACK_ID)?.extract(py))?;
        self.view
            .lock()
            .await
            .remove_delete(callback_id)
            .await
            .into_pyerr()
    }

    pub async fn on_update(&self, callback: Py<PyFunction>, mode: Option<String>) -> PyResult<u32> {
        let loop_cb = self.client.loop_cb.read().await.clone();
        let callback = move |x: ViewOnUpdateResp| {
            let loop_cb = loop_cb.clone();
            let callback = callback.clone();
            async move {
                let aggregate_errors: PyResult<()> = {
                    let callback = callback.clone();
                    Python::with_gil(|py| {
                        match (&x.delta, &loop_cb) {
                            (None, None) => callback.call0(py)?,
                            (None, Some(loop_cb)) => loop_cb.call1(py, (&callback,))?,
                            (Some(delta), None) => {
                                callback.call1(py, (PyBytes::new_bound(py, delta),))?
                            },
                            (Some(delta), Some(loop_cb)) => {
                                loop_cb.call1(py, (callback, PyBytes::new_bound(py, delta)))?
                            },
                        };

                        Ok(())
                    })
                };

                if let Err(err) = aggregate_errors {
                    tracing::warn!("Error in on_update callback: {:?}", err);
                }
            }
        };

        let mode = mode
            .map(|x| OnUpdateMode::from_str(x.as_str()))
            .transpose()
            .into_pyerr()?;

        self.view
            .lock()
            .await
            .on_update(callback.into_box_fn_pin_bix_fut(), OnUpdateOptions { mode })
            .await
            .into_pyerr()
    }

    pub async fn remove_update(&self, callback_id: u32) -> PyResult<()> {
        self.view
            .lock()
            .await
            .remove_update(callback_id)
            .await
            .into_pyerr()
    }

    pub async fn to_arrow(&self, window: Option<Py<PyDict>>) -> PyResult<Py<PyBytes>> {
        let window: ViewWindow =
            Python::with_gil(|py| window.map(|x| depythonize_bound(x.into_bound(py).into_any())))
                .transpose()?
                .unwrap_or_default();
        let arrow = self.view.lock().await.to_arrow(window).await.into_pyerr()?;
        Ok(Python::with_gil(|py| PyBytes::new_bound(py, &arrow).into()))
    }

    pub async fn to_csv(&self, window: Option<Py<PyDict>>) -> PyResult<String> {
        let window: ViewWindow =
            Python::with_gil(|py| window.map(|x| depythonize_bound(x.into_bound(py).into_any())))
                .transpose()?
                .unwrap_or_default();

        self.view.lock().await.to_csv(window).await.into_pyerr()
    }

    pub async fn to_columns_string(&self, window: Option<Py<PyDict>>) -> PyResult<String> {
        let window: ViewWindow =
            Python::with_gil(|py| window.map(|x| depythonize_bound(x.into_bound(py).into_any())))
                .transpose()?
                .unwrap_or_default();

        self.view
            .lock()
            .await
            .to_columns_string(window)
            .await
            .into_pyerr()
    }

    pub async fn to_json_string(&self, window: Option<Py<PyDict>>) -> PyResult<String> {
        let window: ViewWindow =
            Python::with_gil(|py| window.map(|x| depythonize_bound(x.into_bound(py).into_any())))
                .transpose()?
                .unwrap_or_default();

        self.view
            .lock()
            .await
            .to_json_string(window)
            .await
            .into_pyerr()
    }
}
