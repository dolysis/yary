#[derive(Debug, Clone)]
pub(in crate::scanner) struct Key
{
    possible: Option<KeyPossible>,
}

impl Key
{
    pub fn new() -> Self
    {
        Self { possible: None }
    }

    /// A key is possible / .required at the current stream
    /// position
    pub fn possible(&mut self, required: bool)
    {
        self.possible = match required
        {
            true => KeyPossible::Required,
            false => KeyPossible::Yes,
        }
        .into();
    }

    /// A key is impossible / illegal at the current stream
    /// position
    pub fn forbidden(&mut self)
    {
        self.possible = Some(KeyPossible::No)
    }

    /// Is a key allowed at the current position?
    pub fn allowed(&self) -> bool
    {
        self.possible.as_ref().map(|s| s.allowed()).unwrap_or(false)
    }

    /// Is a key required at the current position?
    pub fn required(&self) -> bool
    {
        self.possible
            .as_ref()
            .map(|s| s.required())
            .unwrap_or(false)
    }
}

impl Default for Key
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::scanner) enum KeyPossible
{
    No,
    Yes,
    Required,
}

impl KeyPossible
{
    fn allowed(&self) -> bool
    {
        matches!(self, Self::Yes | Self::Required)
    }

    fn required(&self) -> bool
    {
        matches!(self, Self::Required)
    }
}

impl Default for KeyPossible
{
    fn default() -> Self
    {
        Self::No
    }
}
