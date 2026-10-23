//! `--company-data contoso-setup|contoso-all`: the company of Microsoft's IntegrationTest and Uncategorized test lanes,
//! built the way BCApps' CI builds it (`build/scripts/ImportTestDataInBcContainer.ps1`, `Invoke-ContosoDemoTool`): the
//! steps are `al_testrun::company_data::CompanyData::contoso`.
//!
//! The Contoso Coffee Demo Dataset app is a Microsoft app of the artifact (`apps/Contoso Coffee Demo Dataset.Source.zip`),
//! so nothing is fetched: the project gets it as one more dependency, and the resolver finds it where it finds the test
//! apps' other Microsoft dependencies. Nothing from it is committed to this repository.

use al_deps::select::DepDecl;
use al_deps::version::Version;

/// The app's id (its `app.json`, the same on every release).
pub const APP_ID: &str = "5a0b41e9-7a42-4123-d521-2265186cfb31";
/// The app's name.
pub const APP_NAME: &str = "Contoso Coffee Demo Dataset";

/// The dependency the project gets: any version, the resolver takes the artifact's.
pub fn dependency() -> DepDecl {
    DepDecl {
        id: APP_ID.to_string(),
        name: APP_NAME.to_string(),
        publisher: "Microsoft".to_string(),
        min: Version(0, 0, 0, 0),
        optional: false,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_dependency_names_the_contoso_app_of_any_version() {
        let d = super::dependency();
        assert_eq!(
            (d.name.as_str(), d.publisher.as_str()),
            ("Contoso Coffee Demo Dataset", "Microsoft")
        );
        assert!(!d.optional && d.min == al_deps::version::Version(0, 0, 0, 0));
    }
}
