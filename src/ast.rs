use crate::msbuild;
use std::ops::Deref;

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
        let mut type_id = "";
        let mut pid = "";
        if let Expr::Guid(guid) = project_type {
            type_id = guid;
        }
        if let Expr::Guid(guid) = id {
            pid = guid;
        }
        let mut prj = Project::new(pid, type_id);

        if let Expr::Str(s) = name {
            prj.name = s;
        }
        if let Expr::Str(s) = path {
            prj.path = s;
        }
        prj
    }
}

impl<'input> Version<'input> {
    pub fn new(name: &'input str, ver: &'input str) -> Self {
        Self { name, ver }
    }

    pub fn from(name: &Expr<'input>, val: &Expr<'input>) -> Self {
        let mut n = "";
        let mut v = "";
        if let Expr::Identifier(id) = name {
            n = id;
        }
        if let Expr::DigitOrDot(s) = val {
            v = s;
        }
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
            if let Expr::Str(s) = left.deref() {
                let conf = Configuration::new(*s);
                return Some(conf);
            }
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
        configurations: Vec<Configuration<'input>>,
    ) -> Self {
        Self {
            project_id,
            configurations,
        }
    }

    pub fn from(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            if let Expr::Str(s) = left.deref() {
                let conf = ProjectConfigurations::new(*s);
                return Some(conf);
            }
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
