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
    pub solution_configs: Vec<Conf<'input>>,
    pub project_configs: Vec<ProjectConfigs<'input>>,
}

#[derive(Debug, Copy, Clone)]
pub struct Version<'input> {
    pub name: &'input str,
    pub ver: &'input str,
}

#[derive(Debug, Clone)]
pub struct ProjectConfigs<'input> {
    pub project_id: &'input str,
    pub configs: Vec<Conf<'input>>,
}

#[derive(Debug, Copy, Clone)]
pub struct Conf<'input> {
    pub config: &'input str,
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
            solution_configs: Vec::new(),
            project_configs: Vec::new(),
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

impl<'input> From<&'input str> for Conf<'input> {
    fn from(s: &'input str) -> Self {
        let parts: Vec<&str> = s.split('|').collect();
        let mut config = "";
        let mut platform = "";
        if parts.len() == 2 {
            config = parts[0];
            platform = parts[1];
        }
        Self { config, platform }
    }
}

impl<'input> Conf<'input> {
    pub fn new(configuration: &'input str, platform: &'input str) -> Self {
        Self {
            config: configuration,
            platform,
        }
    }

    pub fn from_expr(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            let conf = Conf::from(left.string());
            return Some(conf);
        }

        None
    }
}

impl<'input> From<&'input str> for ProjectConfigs<'input> {
    fn from(s: &'input str) -> Self {
        let mut it = s.split('.');
        let project_id = it.next().unwrap_or("");

        let mut it = s[project_id.len() + 1..].split('|');
        let config = it.next().unwrap_or("");

        let trail = &s[project_id.len() + config.len() + 2..];
        let mut it = trail.split(".ActiveCfg");
        let mut platform = it.next().unwrap_or("");
        if platform.len() == trail.len() {
            let it = trail.chars().rev();
            let mut dot_count = 0;
            const SKIP_DOTS: usize = 1;

            let break_fn = |ch: &char| -> bool {
                if *ch == '.' {
                    dot_count += 1;
                }
                dot_count <= SKIP_DOTS
            };

            let cut_count = it.take_while(break_fn).count() + 1; // Last dot

            platform = &trail[..trail.len() - cut_count];
        }

        let mut configs = Vec::new();
        let config = Conf::new(config, platform);
        configs.push(config);
        Self {
            project_id,
            configs: configs,
        }
    }
}

impl<'input> ProjectConfigs<'input> {
    pub fn from_id_and_configs(project_id: &'input str, configs: Vec<Conf<'input>>) -> Self {
        let mut configurations = Vec::new();
        configurations.extend(configs);
        Self {
            project_id,
            configs: configurations,
        }
    }

    pub fn new(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            let conf = ProjectConfigs::from(left.string());
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
        let c = Conf::from(s);

        // Assert
        assert_eq!("Release", c.config);
        assert_eq!("Any CPU", c.platform);
    }

    #[test]
    fn from_configuration_empty() {
        // Arrange
        let s = "";

        // Act
        let c = Conf::from(s);

        // Assert
        assert_eq!("", c.config);
        assert_eq!("", c.platform);
    }

    #[test]
    fn from_configuration_incorrect_no_pipe() {
        // Arrange
        let s = "Release Any CPU";

        // Act
        let c = Conf::from(s);

        // Assert
        assert_eq!("", c.config);
        assert_eq!("", c.platform);
    }

    #[test]
    fn from_configuration_incorrect_many_pipes() {
        // Arrange
        let s = "Release|Any CPU|test";

        // Act
        let c = Conf::from(s);

        // Assert
        assert_eq!("", c.config);
        assert_eq!("", c.platform);
    }

    #[test]
    fn from_project_configurations_correct() {
        // Arrange
        let s = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.ActiveCfg";

        // Act
        let c = ProjectConfigs::from(s);

        // Assert
        assert_eq!("{27060CA7-FB29-42BC-BA66-7FC80D498354}", c.project_id);
        assert_eq!(1, c.configs.len());
        assert_eq!("Debug", c.configs[0].config);
        assert_eq!("Any CPU", c.configs[0].platform);
    }

    #[test]
    fn from_project_configurations_config_with_dot() {
        // Arrange
        let s = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug .NET 4.0|Any CPU.ActiveCfg";

        // Act
        let c = ProjectConfigs::from(s);

        // Assert
        assert_eq!("{27060CA7-FB29-42BC-BA66-7FC80D498354}", c.project_id);
        assert_eq!(1, c.configs.len());
        assert_eq!("Debug .NET 4.0", c.configs[0].config);
        assert_eq!("Any CPU", c.configs[0].platform);
    }

    #[test]
    fn from_project_configurations_platform_with_dot_active() {
        // Arrange
        let s = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.ActiveCfg";

        // Act
        let c = ProjectConfigs::from(s);

        // Assert
        assert_eq!("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", c.project_id);
        assert_eq!(1, c.configs.len());
        assert_eq!("Release", c.configs[0].config);
        assert_eq!(".NET", c.configs[0].platform);
    }

    #[test]
    fn from_project_configurations_platform_with_dot_build() {
        // Arrange
        let s = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.Build.0";

        // Act
        let c = ProjectConfigs::from(s);

        // Assert
        assert_eq!("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", c.project_id);
        assert_eq!(1, c.configs.len());
        assert_eq!("Release", c.configs[0].config);
        assert_eq!(".NET", c.configs[0].platform);
    }
}
