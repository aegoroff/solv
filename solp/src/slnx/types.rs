//! Project types resolution for `.slnx` solutions.
//!
//! Mirrors project type table of the reference implementation
//! (Microsoft.VisualStudio.SolutionPersistence): built-in types with their implicit
//! configuration rules and solution defined `ProjectType` elements that may be based on each other.

use super::config::ConfigurationRuleBorrowed;
use super::{Configurations, ProjectType};

/// Max `BasedOn` chain length. Protects from cycles in malformed solutions.
const MAX_BASED_ON_DEPTH: usize = 16;

/// Platform value meaning that project has no platforms
pub const MISSING_PLATFORM: &str = "?";

/// Implicit configuration rules of a built-in project type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInRules {
    /// CLR project: project platform is always Any CPU
    Clr,
    /// Visual C++ project: Any CPU maps to x64 and x86 maps to Win32
    Vc,
    /// Project isn't built by default
    NoBuild,
    /// Project has no platforms
    NoPlatforms,
}

#[derive(Debug)]
pub struct BuiltInType {
    pub type_id: &'static str,
    pub name: Option<&'static str>,
    pub extension: Option<&'static str>,
    pub rules: BuiltInRules,
}

const fn built_in(
    type_id: &'static str,
    name: Option<&'static str>,
    extension: Option<&'static str>,
    rules: BuiltInRules,
) -> BuiltInType {
    BuiltInType {
        type_id,
        name,
        extension,
        rules,
    }
}

const VCXPROJ: &str = "{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}";

/// Order matters: the first type with matching id is used to describe it (so `VC` wins over `.vcxitems`)
const BUILT_IN_TYPES: &[BuiltInType] = &[
    built_in(
        "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}",
        Some("Common C#"),
        None,
        BuiltInRules::Clr,
    ),
    built_in(
        "{778DAE3C-4631-46EA-AA77-85C1314464D9}",
        Some("Common VB"),
        None,
        BuiltInRules::Clr,
    ),
    built_in(
        "{6EC3EE1D-3C4E-46DD-8F32-0CC8E7565705}",
        Some("Common F#"),
        None,
        BuiltInRules::Clr,
    ),
    built_in(
        "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}",
        Some("C#"),
        Some("csproj"),
        BuiltInRules::Clr,
    ),
    built_in(
        "{F184B08F-C81C-45F6-A57F-5ABD9991F28F}",
        Some("VB"),
        Some("vbproj"),
        BuiltInRules::Clr,
    ),
    built_in(
        "{F2A71F9B-5D33-465A-A702-920D77279786}",
        Some("F#"),
        Some("fsproj"),
        BuiltInRules::Clr,
    ),
    built_in(
        "{D954291E-2A0B-460D-934E-DC6B0785DB48}",
        Some("Shared"),
        Some("shproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{E24C65DC-7377-472B-9ABA-BC803B73C61A}",
        Some("Website"),
        Some("webproj"),
        BuiltInRules::Clr,
    ),
    built_in(VCXPROJ, Some("VC"), Some("vcxproj"), BuiltInRules::Vc),
    built_in(VCXPROJ, None, Some("vcxitems"), BuiltInRules::NoBuild),
    built_in(
        "{911E67C6-3D85-4FCE-B560-20A9C3E3FF48}",
        Some("Exe"),
        Some("exe"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{2150E333-8FDC-42A3-9474-1A3956D46DE8}",
        Some("Folder"),
        None,
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{54A90642-561A-4BB1-A94E-469ADEE60C69}",
        Some("Javascript"),
        Some("esproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{9092AA53-FB77-4645-B42D-1CCCA6BD08BD}",
        Some("Node.js"),
        Some("njsproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{151D2E53-A2C4-4D7D-83FE-D05416EBD58E}",
        Some("Deploy"),
        Some("deployproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{54435603-DBB4-11D2-8724-00A0C9A8B90C}",
        Some("Installer"),
        Some("vsproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{930C7802-8A8C-48F9-8165-68863BCCD9DD}",
        Some("Wix"),
        Some("wixproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{00D1A9C2-B5F0-4AF3-8072-F6C62B433612}",
        Some("SQL"),
        Some("sqlproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{0C603C2C-620A-423B-A800-4F3E2F6281F1}",
        Some("U-SQL-DB"),
        Some("usqldbproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{182E2583-ECAD-465B-BB50-91101D7C24CE}",
        Some("U-SQL"),
        Some("usqlproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{F14B399A-7131-4C87-9E4B-1186C45EF12D}",
        Some("SSRS"),
        Some("rptproj"),
        BuiltInRules::NoPlatforms,
    ),
    built_in(
        "{A07B5EB6-E848-4116-A8D0-A826331D98C6}",
        Some("Fabric"),
        Some("sfproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{CC5FD16D-436D-48AD-A40C-5A424C6E3E79}",
        Some("Cloud Computing"),
        Some("ccproj"),
        BuiltInRules::NoBuild,
    ),
    built_in(
        "{E53339B2-1760-4266-BCC7-CA923CBCF16C}",
        Some("Docker"),
        Some("dcproj"),
        BuiltInRules::NoBuild,
    ),
];

const NO_BUILD_RULES: &[ConfigurationRuleBorrowed<'static>] = &[ConfigurationRuleBorrowed {
    solution: None,
    project: Some("false"),
}];

const CLR_PLATFORM_RULES: &[ConfigurationRuleBorrowed<'static>] = &[ConfigurationRuleBorrowed {
    solution: None,
    project: Some("Any CPU"),
}];

const VC_PLATFORM_RULES: &[ConfigurationRuleBorrowed<'static>] = &[
    ConfigurationRuleBorrowed {
        solution: Some("*|Any CPU"),
        project: Some("x64"),
    },
    ConfigurationRuleBorrowed {
        solution: Some("*|x86"),
        project: Some("Win32"),
    },
];

const NO_PLATFORMS_RULES: &[ConfigurationRuleBorrowed<'static>] = &[ConfigurationRuleBorrowed {
    solution: None,
    project: Some(MISSING_PLATFORM),
}];

impl BuiltInType {
    pub fn platform_rules(&self) -> &'static [ConfigurationRuleBorrowed<'static>] {
        match self.rules {
            BuiltInRules::Clr => CLR_PLATFORM_RULES,
            BuiltInRules::Vc => VC_PLATFORM_RULES,
            BuiltInRules::NoPlatforms => NO_PLATFORMS_RULES,
            BuiltInRules::NoBuild => &[],
        }
    }

    pub fn build_rules(&self) -> &'static [ConfigurationRuleBorrowed<'static>] {
        match self.rules {
            BuiltInRules::NoBuild => NO_BUILD_RULES,
            _ => &[],
        }
    }
}

/// Project type found either in solution's `Configurations` or in built-in table
#[derive(Debug, Clone, Copy)]
pub enum TypeRef<'s> {
    Custom(&'s ProjectType),
    BuiltIn(&'static BuiltInType),
}

impl TypeRef<'_> {
    fn name(&self) -> Option<&str> {
        match self {
            TypeRef::Custom(t) => t.name.as_deref(),
            TypeRef::BuiltIn(t) => t.name,
        }
    }
}

/// Returns `true` if value looks like a GUID with or without braces
pub fn is_guid(value: &str) -> bool {
    let value = trim_braces(value);
    value.len() == 36
        && value.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

fn trim_braces(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .unwrap_or(value)
}

fn same_guid(left: &str, right: &str) -> bool {
    trim_braces(left).eq_ignore_ascii_case(trim_braces(right))
}

fn same_extension(left: &str, right: &str) -> bool {
    left.trim_start_matches('.')
        .eq_ignore_ascii_case(right.trim_start_matches('.'))
}

/// Returns built-in type id (normalized form with braces) for the GUID specified
pub fn built_in_type_id(guid: &str) -> Option<&'static str> {
    BUILT_IN_TYPES
        .iter()
        .find(|t| same_guid(t.type_id, guid))
        .map(|t| t.type_id)
}

/// Project type resolver that looks up solution defined types first and built-in types then
pub struct Resolver<'s> {
    custom: &'s [ProjectType],
}

impl<'s> Resolver<'s> {
    pub fn new(configs: Option<&'s Configurations>) -> Self {
        Self {
            custom: configs.map_or(&[], |c| c.project_types.as_slice()),
        }
    }

    /// Solution wide rules i.e. `ProjectType` elements that don't define any type
    pub fn solution_defaults(&self) -> impl Iterator<Item = &'s ProjectType> {
        self.custom.iter().filter(|t| {
            t.type_id.is_none() && t.name.is_none() && t.extension.is_none() && t.based_on.is_none()
        })
    }

    /// Resolves project type by `Type` attribute (name or GUID) and project file extension
    pub fn resolve(&self, type_name: Option<&str>, extension: Option<&str>) -> Option<TypeRef<'s>> {
        let (type_id, type_name) = match type_name {
            Some(value) if is_guid(value) => (Some(value), None),
            other => (None, other.filter(|v| !v.trim().is_empty())),
        };

        self.resolve_custom(type_id, type_name, extension)
            .or_else(|| Self::resolve_built_in(type_id, type_name, extension).map(TypeRef::BuiltIn))
    }

    fn resolve_custom(
        &self,
        type_id: Option<&str>,
        type_name: Option<&str>,
        extension: Option<&str>,
    ) -> Option<TypeRef<'s>> {
        // Implied type from extension is used only if it's compatible with type name or id
        if let Some(extension) = extension
            && let Some(t) = self.custom.iter().find(|t| {
                t.extension
                    .as_deref()
                    .is_some_and(|e| same_extension(e, extension))
            })
            && self.is_compatible(TypeRef::Custom(t), type_id, type_name)
        {
            return Some(TypeRef::Custom(t));
        }

        if let Some(type_name) = type_name
            && let Some(t) = self.custom.iter().find(|t| {
                t.name
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(type_name))
            })
        {
            return Some(TypeRef::Custom(t));
        }

        let type_id = type_id?;
        self.custom
            .iter()
            .find(|t| {
                t.type_id
                    .as_deref()
                    .is_some_and(|id| same_guid(id, type_id))
            })
            .map(TypeRef::Custom)
    }

    fn resolve_built_in(
        type_id: Option<&str>,
        type_name: Option<&str>,
        extension: Option<&str>,
    ) -> Option<&'static BuiltInType> {
        if let Some(extension) = extension
            && let Some(t) = BUILT_IN_TYPES
                .iter()
                .find(|t| t.extension.is_some_and(|e| same_extension(e, extension)))
            && type_id.is_none_or(|id| same_guid(t.type_id, id))
            && type_name.is_none_or(|n| t.name.is_some_and(|tn| tn.eq_ignore_ascii_case(n)))
        {
            return Some(t);
        }

        if let Some(type_name) = type_name
            && let Some(t) = BUILT_IN_TYPES
                .iter()
                .find(|t| t.name.is_some_and(|n| n.eq_ignore_ascii_case(type_name)))
        {
            return Some(t);
        }

        let type_id = type_id?;
        BUILT_IN_TYPES
            .iter()
            .find(|t| same_guid(t.type_id, type_id))
    }

    fn is_compatible(
        &self,
        t: TypeRef<'s>,
        type_id: Option<&str>,
        type_name: Option<&str>,
    ) -> bool {
        type_id.is_none_or(|id| self.type_id(t).is_some_and(|tid| same_guid(tid, id)))
            && type_name.is_none_or(|n| t.name().is_some_and(|tn| tn.eq_ignore_ascii_case(n)))
    }

    /// Type that `BasedOn` attribute of a solution defined type points to
    pub fn based_on(&self, t: TypeRef<'s>) -> Option<TypeRef<'s>> {
        let TypeRef::Custom(custom) = t else {
            return None;
        };
        let based_on = custom.based_on.as_deref()?;
        let (type_id, type_name) = if is_guid(based_on) {
            (Some(based_on), None)
        } else {
            (None, Some(based_on))
        };
        // Type can't be based on itself. Like the reference implementation
        // built-in type may also be referenced by its extension e.g. `.vcxproj`
        self.resolve_custom(type_id, type_name, None)
            .filter(|found| !matches!(found, TypeRef::Custom(f) if std::ptr::eq(*f, custom)))
            .or_else(|| Self::resolve_built_in(type_id, type_name, None).map(TypeRef::BuiltIn))
            .or_else(|| Self::resolve_built_in(None, None, Some(based_on)).map(TypeRef::BuiltIn))
    }

    /// `BasedOn` chain from the most general type to the type itself
    pub fn chain(&self, t: TypeRef<'s>) -> Vec<TypeRef<'s>> {
        let mut chain = vec![t];
        let mut current = t;
        while chain.len() < MAX_BASED_ON_DEPTH {
            let Some(base) = self.based_on(current) else {
                break;
            };
            chain.push(base);
            current = base;
        }
        chain.reverse();
        chain
    }

    /// Type id of the type. If type doesn't define it the `BasedOn` chain is used.
    pub fn type_id(&self, t: TypeRef<'s>) -> Option<&'s str> {
        self.chain(t).iter().rev().find_map(|t| match t {
            TypeRef::Custom(c) => c.type_id.as_deref(),
            TypeRef::BuiltIn(b) => Some(b.type_id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn custom(
        type_id: Option<&str>,
        name: Option<&str>,
        extension: Option<&str>,
        based_on: Option<&str>,
    ) -> ProjectType {
        ProjectType {
            build_types: vec![],
            platforms: vec![],
            builds: vec![],
            deploys: vec![],
            type_id: type_id.map(String::from),
            name: name.map(String::from),
            extension: extension.map(String::from),
            based_on: based_on.map(String::from),
            is_buildable: None,
            supports_platform: None,
        }
    }

    fn configurations(project_types: Vec<ProjectType>) -> Configurations {
        Configurations {
            build_types: vec![],
            platforms: vec![],
            project_types,
        }
    }

    fn built_in_id(t: Option<TypeRef<'_>>) -> Option<&'static str> {
        match t {
            Some(TypeRef::BuiltIn(b)) => Some(b.type_id),
            _ => None,
        }
    }

    #[test_case(None, Some("csproj"), Some("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") ; "csproj extension")]
    #[test_case(None, Some("CSPROJ"), Some("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") ; "extension ignores case")]
    #[test_case(None, Some("vcxproj"), Some(VCXPROJ) ; "vcxproj extension")]
    #[test_case(None, Some("vcxitems"), Some(VCXPROJ) ; "vcxitems extension")]
    #[test_case(None, Some("sqlproj"), Some("{00D1A9C2-B5F0-4AF3-8072-F6C62B433612}") ; "sqlproj extension")]
    #[test_case(None, Some("njsproj"), Some("{9092AA53-FB77-4645-B42D-1CCCA6BD08BD}") ; "njsproj extension")]
    #[test_case(Some("Common C#"), Some("csproj"), Some("{9A19103F-16F7-4668-BE54-9A1E7A4F7556}") ; "name overrides extension")]
    #[test_case(Some("common c#"), Some("csproj"), Some("{9A19103F-16F7-4668-BE54-9A1E7A4F7556}") ; "name ignores case")]
    #[test_case(Some("C#"), Some("csproj"), Some("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") ; "name equal to extension type")]
    #[test_case(Some("VC"), Some("proj"), Some(VCXPROJ) ; "name with unknown extension")]
    #[test_case(Some("9a19103f-16f7-4668-be54-9a1e7a4f7556"), Some("csproj"), Some("{9A19103F-16F7-4668-BE54-9A1E7A4F7556}") ; "guid overrides extension")]
    #[test_case(Some("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}"), None, Some(VCXPROJ) ; "guid with braces")]
    #[test_case(Some("Python"), Some("pyproj"), None ; "unknown name")]
    #[test_case(None, Some("pyproj"), None ; "unknown extension")]
    #[test_case(Some("11111111-2222-3333-4444-555555555555"), Some("csproj"), None ; "unknown guid")]
    fn resolve_built_in_types(
        type_name: Option<&str>,
        extension: Option<&str>,
        expected: Option<&str>,
    ) {
        // Arrange
        let resolver = Resolver::new(None);

        // Act
        let actual = resolver.resolve(type_name, extension);

        // Assert
        assert_eq!(built_in_id(actual), expected);
    }

    #[test]
    fn all_built_in_types_have_description() {
        // Arrange

        // Act
        let undescribed = BUILT_IN_TYPES
            .iter()
            .filter(|t| crate::msbuild::describe_project(t.type_id) == t.type_id)
            .map(|t| t.type_id)
            .collect::<Vec<_>>();

        // Assert
        assert!(undescribed.is_empty(), "{undescribed:?}");
    }

    #[test]
    fn resolve_prefers_solution_defined_type_by_extension() {
        // Arrange
        let configs = configurations(vec![custom(None, Some("My"), Some(".csproj"), Some("VC"))]);
        let resolver = Resolver::new(Some(&configs));

        // Act
        let actual = resolver.resolve(None, Some("csproj")).unwrap();

        // Assert
        assert!(matches!(actual, TypeRef::Custom(t) if t.name.as_deref() == Some("My")));
        assert_eq!(resolver.type_id(actual), Some(VCXPROJ));
    }

    #[test]
    fn resolve_solution_defined_type_by_name_and_guid() {
        // Arrange
        let configs = configurations(vec![custom(
            Some("11111111-2222-3333-4444-555555555555"),
            Some("Custom"),
            None,
            None,
        )]);
        let resolver = Resolver::new(Some(&configs));

        // Act
        let by_name = resolver.resolve(Some("custom"), Some("proj"));
        let by_guid =
            resolver.resolve(Some("{11111111-2222-3333-4444-555555555555}"), Some("proj"));

        // Assert
        assert!(matches!(by_name, Some(TypeRef::Custom(_))));
        assert!(matches!(by_guid, Some(TypeRef::Custom(_))));
    }

    #[test]
    fn chain_goes_from_base_to_derived() {
        // Arrange
        let configs = configurations(vec![
            custom(None, Some("Derived"), None, Some("Base")),
            custom(None, Some("Base"), None, Some("C#")),
        ]);
        let resolver = Resolver::new(Some(&configs));
        let derived = resolver.resolve(Some("Derived"), None).unwrap();

        // Act
        let chain = resolver.chain(derived);

        // Assert
        let names = chain.iter().map(TypeRef::name).collect::<Vec<_>>();
        assert_eq!(names, vec![Some("C#"), Some("Base"), Some("Derived")]);
        assert_eq!(
            resolver.type_id(derived),
            Some("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}")
        );
    }

    #[test_case(".vcxproj" ; "extension with dot")]
    #[test_case("VCXPROJ" ; "extension ignores case and dot")]
    fn based_on_built_in_type_by_extension(based_on: &str) {
        // Arrange
        let configs = configurations(vec![custom(None, None, Some("myproj"), Some(based_on))]);
        let resolver = Resolver::new(Some(&configs));
        let derived = resolver.resolve(None, Some("myproj")).unwrap();

        // Act
        let actual = resolver.type_id(derived);

        // Assert
        assert_eq!(actual, Some(VCXPROJ));
    }

    #[test]
    fn chain_with_cycle_is_limited() {
        // Arrange
        let configs = configurations(vec![
            custom(None, Some("A"), None, Some("B")),
            custom(None, Some("B"), None, Some("A")),
        ]);
        let resolver = Resolver::new(Some(&configs));
        let a = resolver.resolve(Some("A"), None).unwrap();

        // Act
        let chain = resolver.chain(a);

        // Assert
        assert_eq!(chain.len(), MAX_BASED_ON_DEPTH);
        assert_eq!(resolver.type_id(a), None);
    }

    #[test]
    fn solution_defaults_are_types_without_identity() {
        // Arrange
        let configs = configurations(vec![
            custom(None, None, None, None),
            custom(None, Some("Named"), None, None),
        ]);
        let resolver = Resolver::new(Some(&configs));

        // Act
        let defaults = resolver.solution_defaults().count();

        // Assert
        assert_eq!(defaults, 1);
    }

    #[test_case("9A19103F-16F7-4668-BE54-9A1E7A4F7556", true ; "plain")]
    #[test_case("{9a19103f-16f7-4668-be54-9a1e7a4f7556}", true ; "braces lowercase")]
    #[test_case("C#", false ; "name")]
    #[test_case("9A19103F16F74668BE549A1E7A4F7556", false ; "without dashes")]
    #[test_case("{9A19103F-16F7-4668-BE54-9A1E7A4F755Z}", false ; "not hex")]
    fn is_guid_cases(value: &str, expected: bool) {
        // Arrange

        // Act
        let actual = is_guid(value);

        // Assert
        assert_eq!(actual, expected);
    }
}
