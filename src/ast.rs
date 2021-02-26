use crate::msbuild;

#[derive(Debug)]
pub enum Expr<'input> {
    Comment(&'input str),
    DigitOrDot(&'input str),
    Guid(&'input str),
    Identifier(&'input str),
    Str(&'input str),
    Version(Box<Expr<'input>>, Box<Expr<'input>>),
    FirstLine(Box<Expr<'input>>),
    Global(Vec<Expr<'input>>),
    Project(Box<Expr<'input>>, Vec<Expr<'input>>),
    ProjectBegin(
        Box<Expr<'input>>,
        Box<Expr<'input>>,
        Box<Expr<'input>>,
        Box<Expr<'input>>,
    ),
    Section(Box<Expr<'input>>, Vec<Expr<'input>>),
    SectionBegin(Vec<Expr<'input>>, Box<Expr<'input>>),
    SectionContent(Box<Expr<'input>>, Box<Expr<'input>>),
    SectionKey(Box<Expr<'input>>),
    SectionValue(Box<Expr<'input>>),
}

impl<'input> Expr<'input> {
    pub fn identifier(&self) -> &'input str {
        if let Expr::Identifier(s) = self {
            return *s;
        }
        ""
    }

    pub fn digit_or_dot(&self) -> &'input str {
        if let Expr::DigitOrDot(s) = self {
            return *s;
        }
        ""
    }

    pub fn string(&self) -> &'input str {
        if let Expr::Str(s) = self {
            return *s;
        }
        ""
    }

    pub fn guid(&self) -> &'input str {
        if let Expr::Guid(s) = self {
            return *s;
        }
        ""
    }

    pub fn is_section(&self, name: &str) -> bool {
        if let Expr::SectionBegin(names, _) = self {
            return names.iter().any(|n| n.identifier() == name);
        }

        false
    }

    pub fn section_content(&self, name: &str) -> Option<&'input Vec<Expr>> {
        if let Expr::Section(begin, content) = self {
            if begin.is_section(name) {
                return Some(content);
            }
            return None;
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct Solution<'input> {
    pub format: &'input str,
    pub product: &'input str,
    pub projects: Vec<Project<'input>>,
    pub versions: Vec<Version<'input>>,
    pub configurations: Vec<Configuration<'input>>,
    pub project_configurations: Vec<ProjectConfigurations<'input>>,
}

#[derive(Debug, Copy, Clone)]
pub struct Version<'input> {
    pub name: &'input str,
    pub ver: &'input str,
}

#[derive(Debug, Clone)]
pub struct ProjectConfigurations<'input> {
    pub project_id: &'input str,
    pub configurations: Vec<Configuration<'input>>,
}

#[derive(Debug, Copy, Clone)]
pub struct Configuration<'input> {
    pub configuration: &'input str,
    pub platform: &'input str,
}

#[derive(Debug, Copy, Clone)]
pub struct Project<'input> {
    pub type_id: &'input str,
    pub type_descr: &'input str,
    pub id: &'input str,
    pub name: &'input str,
    pub path: &'input str,
}

impl<'input> Default for Solution<'input> {
    fn default() -> Self {
        Self {
            format: "",
            product: "",
            projects: Vec::new(),
            versions: Vec::new(),
            configurations: Vec::new(),
            project_configurations: Vec::new(),
        }
    }
}

impl<'input> Project<'input> {
    pub fn new(id: &'input str, type_id: &'input str) -> Self {
        let type_descr;
        if let Some(type_name) = msbuild::PROJECT_TYPES.get(type_id) {
            type_descr = *type_name;
        } else {
            type_descr = type_id;
        }

        Self {
            id,
            type_id,
            type_descr,
            name: "",
            path: "",
        }
    }

    pub fn from_begin(head: &Expr<'input>) -> Option<Self> {
        if let Expr::ProjectBegin(project_type, name, path, id) = head {
            let prj = Project::from(project_type, name, path, id);
            return Some(prj);
        }
        None
    }

    pub fn from(
        project_type: &Expr<'input>,
        name: &Expr<'input>,
        path: &Expr<'input>,
        id: &Expr<'input>,
    ) -> Self {
        let type_id = project_type.guid();
        let pid = id.guid();

        let mut prj = Project::new(pid, type_id);
        prj.name = name.string();
        prj.path = path.string();

        prj
    }
}

impl<'input> Version<'input> {
    pub fn new(name: &'input str, ver: &'input str) -> Self {
        Self { name, ver }
    }

    pub fn from(name: &Expr<'input>, val: &Expr<'input>) -> Self {
        let n = name.identifier();
        let v = val.digit_or_dot();
        Version::new(n, v)
    }
}

impl<'input> From<&'input str> for Configuration<'input> {
    fn from(s: &'input str) -> Self {
        let parts: Vec<&str> = s.split('|').collect();
        let mut configuration = "";
        let mut platform = "";
        if parts.len() == 2 {
            configuration = parts[0];
            platform = parts[1];
        }
        Self {
            configuration,
            platform,
        }
    }
}

impl<'input> Configuration<'input> {
    pub fn new(configuration: &'input str, platform: &'input str) -> Self {
        Self {
            configuration,
            platform,
        }
    }

    pub fn from_expr(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            let conf = Configuration::from(left.string());
            return Some(conf);
        }

        None
    }
}

impl<'input> From<&'input str> for ProjectConfigurations<'input> {
    fn from(s: &'input str) -> Self {
        let mut it = s.split('.');
        let project_id = it.next().unwrap_or("");

        let mut it = s[project_id.len() + 1..].split('|');
        let config = it.next().unwrap_or("");

        let trail = &s[project_id.len() + config.len() + 2..];
        let mut it = trail.split(".ActiveCfg");
        let mut platform = it.next().unwrap_or("");
        if platform.len() == trail.len() {
            let mut it = trail.chars().rev();
            let mut dot_count = 0;
            let mut cut_count = 0;
            while dot_count < 2 {
                cut_count += 1;
                match it.next() {
                    Some('.') => dot_count += 1,
                    None => break,
                    _ => {}
                }
            }
            platform = &trail[..trail.len() - cut_count];
        }

        let mut configurations = Vec::new();
        let configuration = Configuration::new(config, platform);
        configurations.push(configuration);
        Self {
            project_id,
            configurations,
        }
    }
}

impl<'input> ProjectConfigurations<'input> {
    pub fn from_id_and_configurations(
        project_id: &'input str,
        configs: Vec<Configuration<'input>>,
    ) -> Self {
        let mut configurations = Vec::new();
        configurations.extend(configs);
        Self {
            project_id,
            configurations,
        }
    }

    pub fn new(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            let conf = ProjectConfigurations::from(left.string());
            return Some(conf);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_configuration_correct() {
        // Arrange
        let s = "Release|Any CPU";

        // Act
        let c = Configuration::from(s);

        // Assert
        assert_eq!("Release", c.configuration);
        assert_eq!("Any CPU", c.platform);
    }

    #[test]
    fn from_configuration_empty() {
        // Arrange
        let s = "";

        // Act
        let c = Configuration::from(s);

        // Assert
        assert_eq!("", c.configuration);
        assert_eq!("", c.platform);
    }

    #[test]
    fn from_configuration_incorrect_no_pipe() {
        // Arrange
        let s = "Release Any CPU";

        // Act
        let c = Configuration::from(s);

        // Assert
        assert_eq!("", c.configuration);
        assert_eq!("", c.platform);
    }

    #[test]
    fn from_configuration_incorrect_many_pipes() {
        // Arrange
        let s = "Release|Any CPU|test";

        // Act
        let c = Configuration::from(s);

        // Assert
        assert_eq!("", c.configuration);
        assert_eq!("", c.platform);
    }

    #[test]
    fn from_project_configurations_correct() {
        // Arrange
        let s = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.ActiveCfg";

        // Act
        let c = ProjectConfigurations::from(s);

        // Assert
        assert_eq!("{27060CA7-FB29-42BC-BA66-7FC80D498354}", c.project_id);
        assert_eq!(1, c.configurations.len());
        assert_eq!("Debug", c.configurations[0].configuration);
        assert_eq!("Any CPU", c.configurations[0].platform);
    }

    #[test]
    fn from_project_configurations_config_with_dot() {
        // Arrange
        let s = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug .NET 4.0|Any CPU.ActiveCfg";

        // Act
        let c = ProjectConfigurations::from(s);

        // Assert
        assert_eq!("{27060CA7-FB29-42BC-BA66-7FC80D498354}", c.project_id);
        assert_eq!(1, c.configurations.len());
        assert_eq!("Debug .NET 4.0", c.configurations[0].configuration);
        assert_eq!("Any CPU", c.configurations[0].platform);
    }

    #[test]
    fn from_project_configurations_platform_with_dot_active() {
        // Arrange
        let s = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.ActiveCfg";

        // Act
        let c = ProjectConfigurations::from(s);

        // Assert
        assert_eq!("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", c.project_id);
        assert_eq!(1, c.configurations.len());
        assert_eq!("Release", c.configurations[0].configuration);
        assert_eq!(".NET", c.configurations[0].platform);
    }

    #[test]
    fn from_project_configurations_platform_with_dot_build() {
        // Arrange
        let s = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.Build.0";

        // Act
        let c = ProjectConfigurations::from(s);

        // Assert
        assert_eq!("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", c.project_id);
        assert_eq!(1, c.configurations.len());
        assert_eq!("Release", c.configurations[0].configuration);
        assert_eq!(".NET", c.configurations[0].platform);
    }
}
