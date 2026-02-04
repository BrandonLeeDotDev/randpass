#[derive(Debug, Default)]
pub struct CliFlags {
    pub help: bool,
    pub version: bool,
    pub bytes: bool,
    pub urandom: bool,
    pub clipboard: bool,
    pub saved: bool,
    pub default: bool,
    pub command: bool,
    pub quiet: bool,
    pub no_special: bool,
    pub hex: bool,
    pub length: Option<usize>,
    pub number: Option<usize>,
    pub number_raw: Option<String>,
    pub special: Option<String>,
    pub output: Option<String>,
}

impl CliFlags {
    pub fn has_explicit_args(&self) -> bool {
        self.length.is_some()
            || self.number.is_some()
            || self.saved
            || self.default
            || self.no_special
            || self.hex
            || self.special.is_some()
            || self.output.is_some()
    }
}
