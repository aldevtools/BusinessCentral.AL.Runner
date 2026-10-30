//! Identifiers the runtime makes up for what a service tier would assign.

/// The package id of the app with id `app` (16 bytes in text order).
///
/// A real tier assigns the package ids when it publishes an app, and nothing in a process can know
/// them. The contract the corpus pins is only this: the id is not empty, every object of one app
/// carries the same one (AllObj and AllObjWithCaption agree for one object), and different apps
/// have different ones (record/TestAllObjOwningAppPerKind.al:109-132, TestAllObjVirtualTable.al
/// "OwnTable_PackageIdsEqualAllObj"). A function of the app id gives exactly that, and is the same
/// in every run. The empty app (an object nothing owns) has the empty id.
pub(crate) fn package_id(app: [u8; 16]) -> [u8; 16] {
    al_builtins::moduleinfo::package_id_of(app)
}

/// The Media id of an app's logo (Published Application's Logo): not null, the same on every read
/// (onprem session/UsAppLogo.al:93-96), and one per app. Its value is not pinned.
pub(crate) fn logo_media_id(app: [u8; 16]) -> [u8; 16] {
    let mut salted = package_id(app);
    salted[14] ^= 0x4c;
    package_id(salted)
}

/// The runtime package id of an app: its package id for the app under test, another one for a dependency app (Base
/// Application), as AllObj reports it for the app's objects and Published Application lists it
/// (metadata/W6AllObj.al:AllObj_PackageIds_ThisApp_BaseApp_Platform).
pub(crate) fn runtime_package_id(app: [u8; 16], dependency: bool) -> [u8; 16] {
    let package = package_id(app);
    if !dependency || app == [0; 16] {
        return package;
    }
    let mut salted = package;
    salted[15] ^= 0x5a;
    package_id(salted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_package_id_follows_the_app_and_is_never_empty_for_a_real_app() {
        let (a, b) = ([1u8; 16], [2u8; 16]);
        assert_ne!(package_id(a), [0; 16]);
        assert_eq!(package_id(a), package_id(a));
        assert_ne!(package_id(a), package_id(b));
        assert_ne!(package_id(a), a);
        assert_eq!(package_id([0; 16]), [0; 16]);
    }
}
