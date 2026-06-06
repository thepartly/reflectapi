use anyhow::{anyhow, bail, Context as _, Result};
use dprint_core::configuration::{ConfigKeyMap, ConfigKeyValue, GlobalConfiguration};
use dprint_core::plugins::FormatConfigId;
use sha2::{Digest as _, Sha256};
use wasmi::{Engine, Instance, Memory, Module, Store, TypedFunc};

const RUFF_WASM: &[u8] = include_bytes!("python/ruff-0.7.15.wasm");
const RUFF_WASM_SHA256: &str = "889d8dc7e8ef0d03437c164b9ac95c5fcdbb67bb277322092d50154027d90a8b";
const GENERATED_FILENAME: &str = "generated.py";

pub fn format_python_code(code: &str) -> Result<String> {
    let mut formatter = RuffWasmFormatter::new()?;
    formatter.format(GENERATED_FILENAME, code)
}

struct RuffWasmFormatter {
    store: Store<()>,
    instance: Instance,
    memory: Memory,
    current_config_id: Option<FormatConfigId>,
    buffer_size: usize,
}

impl RuffWasmFormatter {
    fn new() -> Result<Self> {
        verify_wasm_artifact()?;

        let engine = Engine::default();
        let module =
            Module::new(&engine, RUFF_WASM).context("failed to compile bundled Ruff wasm")?;
        let mut store = Store::new(&engine, ());
        let linker = wasmi::Linker::new(&engine);
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .context("failed to instantiate bundled Ruff wasm")?;
        let memory = instance
            .get_memory(&store, "memory")
            .context("bundled Ruff wasm does not export memory")?;
        let mut formatter = Self {
            store,
            instance,
            memory,
            current_config_id: None,
            buffer_size: 0,
        };
        formatter.ensure_v3_plugin()?;
        formatter.buffer_size = formatter.call0::<u32>("get_wasm_memory_buffer_size")? as usize;
        if formatter.buffer_size == 0 {
            bail!("bundled Ruff wasm reported a zero-sized transfer buffer");
        }
        Ok(formatter)
    }

    fn format(&mut self, file_path: &str, code: &str) -> Result<String> {
        let config = FormatConfig::default_python();
        self.ensure_config(&config)?;

        self.send_string(file_path)?;
        self.call_void0("set_file_path")?;

        self.send_bytes(code.as_bytes())?;
        match self.call0::<u32>("format")? {
            0 => Ok(ensure_final_newline(code.to_string())),
            1 => {
                let len = self.call0::<u32>("get_formatted_text")? as usize;
                Ok(ensure_final_newline(String::from_utf8(
                    self.receive_bytes(len)?,
                )?))
            }
            2 => {
                let len = self.call0::<u32>("get_error_text")? as usize;
                let error_text = String::from_utf8(self.receive_bytes(len)?)?;
                Err(anyhow!(
                    "failed to format generated Python code with bundled Ruff: {error_text}"
                ))
            }
            response => bail!("bundled Ruff wasm returned unknown format response {response}"),
        }
    }

    fn ensure_v3_plugin(&mut self) -> Result<()> {
        let schema_version = self.call0::<u32>("get_plugin_schema_version")?;
        if schema_version != 3 {
            bail!("bundled Ruff wasm uses unsupported dprint plugin schema {schema_version}");
        }
        Ok(())
    }

    fn ensure_config(&mut self, config: &FormatConfig) -> Result<()> {
        if self.current_config_id == Some(config.id) {
            return Ok(());
        }

        self.current_config_id = None;
        self.send_string(&serde_json::to_string(&config.global)?)?;
        self.call_void0("set_global_config")?;
        self.send_string(&serde_json::to_string(&config.plugin)?)?;
        self.call_void0("set_plugin_config")?;
        self.current_config_id = Some(config.id);
        Ok(())
    }

    fn send_string(&mut self, text: &str) -> Result<()> {
        self.send_bytes(text.as_bytes())
    }

    fn send_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.call_void1("clear_shared_bytes", bytes.len() as u32)?;

        let mut offset = 0;
        while offset < bytes.len() {
            let len = std::cmp::min(bytes.len() - offset, self.buffer_size);
            self.write_to_buffer(&bytes[offset..offset + len])?;
            self.call_void1("add_to_shared_bytes_from_buffer", len as u32)?;
            offset += len;
        }
        Ok(())
    }

    fn receive_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut bytes = vec![0; len];
        let mut offset = 0;
        while offset < len {
            let read_len = std::cmp::min(len - offset, self.buffer_size);
            self.call_void2(
                "set_buffer_with_shared_bytes",
                offset as u32,
                read_len as u32,
            )?;
            self.read_from_buffer(&mut bytes[offset..offset + read_len])?;
            offset += read_len;
        }
        Ok(bytes)
    }

    fn write_to_buffer(&mut self, bytes: &[u8]) -> Result<()> {
        let ptr = self.call0::<u32>("get_wasm_memory_buffer")?;
        self.memory
            .write(&mut self.store, ptr as usize, bytes)
            .context("failed to write bytes to bundled Ruff wasm")?;
        Ok(())
    }

    fn read_from_buffer(&mut self, bytes: &mut [u8]) -> Result<()> {
        let ptr = self.call0::<u32>("get_wasm_memory_buffer")?;
        self.memory
            .read(&self.store, ptr as usize, bytes)
            .context("failed to read bytes from bundled Ruff wasm")?;
        Ok(())
    }

    fn call0<Rets>(&mut self, name: &str) -> Result<Rets>
    where
        Rets: wasmi::WasmResults,
    {
        Ok(self
            .get_export::<(), Rets>(name)?
            .call(&mut self.store, ())?)
    }

    fn call_void0(&mut self, name: &str) -> Result<()> {
        Ok(self.get_export::<(), ()>(name)?.call(&mut self.store, ())?)
    }

    fn call_void1(&mut self, name: &str, value: u32) -> Result<()> {
        Ok(self
            .get_export::<u32, ()>(name)?
            .call(&mut self.store, value)?)
    }

    fn call_void2(&mut self, name: &str, value_one: u32, value_two: u32) -> Result<()> {
        Ok(self
            .get_export::<(u32, u32), ()>(name)?
            .call(&mut self.store, (value_one, value_two))?)
    }

    fn get_export<Args, Rets>(&self, name: &str) -> Result<TypedFunc<Args, Rets>>
    where
        Args: wasmi::WasmParams,
        Rets: wasmi::WasmResults,
    {
        self.instance
            .get_typed_func::<Args, Rets>(&self.store, name)
            .with_context(|| format!("bundled Ruff wasm does not export {name}"))
    }
}

struct FormatConfig {
    id: FormatConfigId,
    plugin: ConfigKeyMap,
    global: GlobalConfiguration,
}

impl FormatConfig {
    fn default_python() -> Self {
        let mut plugin = ConfigKeyMap::new();
        plugin.insert("lineLength".to_string(), ConfigKeyValue::Number(88));
        Self {
            id: FormatConfigId::from_raw(1),
            plugin,
            global: GlobalConfiguration::default(),
        }
    }
}

fn verify_wasm_artifact() -> Result<()> {
    let digest = Sha256::digest(RUFF_WASM);
    let actual = format!("{digest:x}");
    if actual != RUFF_WASM_SHA256 {
        bail!("bundled Ruff wasm checksum mismatch: expected {RUFF_WASM_SHA256}, got {actual}");
    }
    Ok(())
}

fn ensure_final_newline(mut code: String) -> String {
    if !code.ends_with('\n') {
        code.push('\n');
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_python_with_bundled_ruff() {
        let formatted = format_python_code("x={\"a\":1,\"b\":2}\n").unwrap();
        assert_eq!(formatted, "x = {\"a\": 1, \"b\": 2}\n");
    }

    #[test]
    fn reports_syntax_errors() {
        let error = format_python_code("def nope(:\n").unwrap_err().to_string();
        assert!(error.contains("failed to format generated Python code with bundled Ruff"));
    }

    #[test]
    fn wasm_artifact_checksum_matches() {
        verify_wasm_artifact().unwrap();
    }
}
