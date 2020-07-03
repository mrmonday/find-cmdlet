/// Cmdlet information for indexing
pub struct Cmdlet {
    /// Module the cmdlet came from
    pub module: String,

    /// Module version for the cmdlet
    pub module_version: String,

    /// Full name of the cmdlet, eg. Get-Something
    pub name: String,

    /// Help URL for the cmdlet
    pub url: String,

    /// List of tags to associate with the cmdlet
    pub tags: Vec<String>,

    /// Synopsis help text for the cmdlet
    pub synopsis: String,

    /// Syntax string for the cmdlet
    pub syntax: String,

    /// Full description help text for the cmdlet
    pub description: String,

    /// Help text notes for the cmdlet
    pub notes: String,
}
