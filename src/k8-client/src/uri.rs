
use crate::http::Uri;


use k8_obj_metadata::Crd;
use k8_obj_metadata::Spec;

use k8_obj_metadata::options::ListOptions;
use k8_metadata_client::NameSpace;

/// items uri
pub fn item_uri<S>(host: &str, name: &str, namespace: &str, sub_resource: Option<&str>) -> Uri
where
    S: Spec,
{
    let ns = if S::NAME_SPACED {
        NameSpace::Named(namespace.to_owned())
    } else {
        NameSpace::All
    };
    let crd = S::metadata();
    let prefix = prefix_uri(crd, host, ns , None);
    let uri_value = format!("{}/{}{}", prefix, name, sub_resource.unwrap_or(""));
    let uri: Uri = uri_value.parse().unwrap();
    uri
}

/// items uri
pub fn items_uri<S>(host: &str, namespace: NameSpace, list_options: Option<ListOptions>) -> Uri
where
    S: Spec,
{
    let ns = if S::NAME_SPACED {
        namespace
    } else {
        NameSpace::All
    };
    let crd = S::metadata();
    let uri_value = prefix_uri(crd, host, ns, list_options);
    let uri: Uri = uri_value.parse().unwrap();
    uri
}




/// related to query parameters and uri
///
///
///
/// generate prefix for given crd
/// if crd group is core then /api is used otherwise /apis + group

pub fn prefix_uri<N>(crd: &Crd, host: &str, ns: N, options: Option<ListOptions>) -> String
    where N: Into<NameSpace>
{
    let namespace = ns.into();
    let version = crd.version;
    let plural = crd.names.plural;
    let group = crd.group.as_ref();
    let api_prefix = match group {
        "core" => "api".to_owned(),
        _ => format!("apis/{}", group),
    };

    let query = if let Some(opt) = options {
        let mut query = "?".to_owned();
        let qs = serde_qs::to_string(&opt).unwrap();
        query.push_str(&qs);
        query
    } else {
        "".to_owned()
    };

    if namespace.is_all() {
        format!(
            "{}/{}/{}/{}{}",
            host, api_prefix, version, plural, query
        )
        
    } else {
        format!(
            "{}/{}/{}/namespaces/{}/{}{}",
            host, api_prefix, version, namespace.named(), plural, query
        )
    }
    
}

#[cfg(test)]
mod test {

    use k8_obj_metadata::DEFAULT_NS;
    use k8_obj_metadata::Crd;
    use k8_obj_metadata::CrdNames;

    use super::prefix_uri;
    use super::ListOptions;
    

    const G1: Crd = Crd {
        group: "test.com",
        version: "v1",
        names: CrdNames {
            kind: "Item",
            plural: "items",
            singular: "item",
        },
    };

    const C1: Crd = Crd {
        group: "core",
        version: "v1",
        names: CrdNames {
            kind: "Item",
            plural: "items",
            singular: "item",
        },
    };

    

    #[test]
    fn test_api_prefix_group() {
        let uri = prefix_uri(&G1, "https://localhost", DEFAULT_NS, None);
        assert_eq!(
            uri,
            "https://localhost/apis/test.com/v1/namespaces/default/items"
        );
    }

    #[test]
    fn test_api_prefix_core() {
        let uri = prefix_uri(&C1, "https://localhost", DEFAULT_NS, None);
        assert_eq!(uri, "https://localhost/api/v1/namespaces/default/items");
    }

    #[test]
    fn test_api_prefix_watch() {
        let opt = ListOptions {
            watch: Some(true),
            ..Default::default()
        };
        let uri = prefix_uri(&C1, "https://localhost", DEFAULT_NS, Some(opt));
        assert_eq!(
            uri,
            "https://localhost/api/v1/namespaces/default/items?watch=true"
        );
    }

    #[test]
    fn test_list_query() {
        let opt = ListOptions {
            pretty: Some(true),
            watch: Some(true),
            ..Default::default()
        };

        let qs = serde_qs::to_string(&opt).unwrap();
        assert_eq!(qs, "pretty=true&watch=true")
    }

}



/*
#[cfg(test)]
mod test {

    use k8_obj_metadata::item_uri;
    use k8_obj_metadata::items_uri;
    use k8_obj_metadata::DEFAULT_NS;
    use crate::pod::PodSpec;

    #[test]
    fn test_pod_item_uri() {
        let uri = item_uri::<PodSpec>("https://localhost", "test", DEFAULT_NS, None);
        assert_eq!(
            uri,
            "https://localhost/api/v1/namespaces/default/pods/test"
        );
    }

    #[test]
    fn test_pod_items_uri() {
        let uri = items_uri::<PodSpec>("https://localhost", DEFAULT_NS, None);
        assert_eq!(
            uri,
            "https://localhost/api/v1/namespaces/default/pods"
        );
    }


}

*/