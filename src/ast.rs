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
            return names.into_iter().any(|n| n.identifier() == name);
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

impl<'input> Solution<'input> {
    pub fn new() -> Self {
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

impl<'input> Configuration<'input> {
    pub fn new(s: &'input str) -> Self {
        let parts: Vec<&str> = s.split("|").collect();
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

    pub fn from(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            let conf = Configuration::new(left.string());
            return Some(conf);
        }

        None
    }
}

impl<'input> ProjectConfigurations<'input> {
    pub fn new(s: &'input str) -> Self {
        let parts: Vec<&str> = s.split(".").collect();
        let mut project_id = "";
        let mut config_and_platform = "";
        if parts.len() >= 2 {
            project_id = parts[0];
            config_and_platform = parts[1];
        }
        let mut configurations = Vec::new();
        let configuration = Configuration::new(config_and_platform);
        configurations.push(configuration);
        Self {
            project_id,
            configurations,
        }
    }

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

    pub fn from(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            let conf = ProjectConfigurations::new(left.string());
            return Some(conf);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_configuration_correct() {
        // Arrange
        let s = "Release|Any CPU";

        // Act
        let c = Configuration::new(s);

        // Assert
        assert_eq!("Release", c.configuration);
        assert_eq!("Any CPU", c.platform);
    }

    #[test]
    fn new_configuration_empty() {
        // Arrange
        let s = "";

        // Act
        let c = Configuration::new(s);

        // Assert
        assert_eq!("", c.configuration);
        assert_eq!("", c.platform);
    }

    #[test]
    fn new_configuration_incorrect_no_pipe() {
        // Arrange
        let s = "Release Any CPU";

        // Act
        let c = Configuration::new(s);

        // Assert
        assert_eq!("", c.configuration);
        assert_eq!("", c.platform);
    }

    #[test]
    fn new_configuration_incorrect_many_pipes() {
        // Arrange
        let s = "Release|Any CPU|test";

        // Act
        let c = Configuration::new(s);

        // Assert
        assert_eq!("", c.configuration);
        assert_eq!("", c.platform);
    }
}
