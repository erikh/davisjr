use crate::{Error, Params};

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub(crate) enum RoutePart {
    Wildcard,
    PathComponent(String),
    Param(String),
    Leader,
}

#[derive(Debug, Clone, PartialOrd)]
pub(crate) struct Path(Vec<RoutePart>);

impl Eq for Path {}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl Path {
    pub(crate) fn new(path: String) -> Result<Self, Error> {
        let mut parts = Self::default();

        let path = path.trim_end_matches("/");

        if !path.contains("/") {
            return Ok(Self::default());
        }

        let args = path.split("/");
        let mut wildcard = false;

        for arg in args {
            if arg.starts_with(":") {
                // is param
                if wildcard {
                    return Err(Error::new(
                        "params may not immediately follow wildcards due to ambiguity",
                    ));
                } else {
                    parts.push(RoutePart::Param(arg.trim_start_matches(":").to_string()));
                };
            } else if arg == "*" {
                if wildcard {
                    return Err(Error::new(
                        "no more than one wildcard may be used in a path",
                    ));
                } else {
                    parts.push(RoutePart::Wildcard);
                    wildcard = true;
                };
            } else if arg == "" {
                // skip empties. this will push additional leaders if there is an duplicate slash
                // (e.g.: `//one/two`), which will fail on matching; we don't want to support this
                // syntax in the router.
            } else {
                // is not param
                parts.push(RoutePart::PathComponent(arg.to_string()));
                wildcard = false;
            }
        }

        Ok(parts)
    }

    pub(crate) fn push(&mut self, arg: RoutePart) -> Self {
        self.0.push(arg);
        self.clone()
    }

    /// This method lists all the params available to the path; useful for debugging.
    #[allow(dead_code)]
    pub(crate) fn params(&self) -> Vec<String> {
        let mut params = Vec::new();
        for arg in self.0.clone() {
            match arg {
                RoutePart::Param(p) => params.push(p),
                _ => {}
            }
        }

        params
    }

    pub(crate) fn extract(&self, provided: String) -> Result<Params, Error> {
        let trimmed = provided.trim_end_matches("/");

        if trimmed == "" && self.eq(&Self::default()) {
            return Ok(Params::default());
        }

        if !self.matches(provided.clone())? {
            return Err(Error::new("route does not match"));
        }

        let mut params = Params::default();

        let parts: Vec<String> = trimmed
            .split("/")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let mut wildcard_vec = Vec::new();
        let mut wildcard = false;
        let mut i = 0;

        for part in parts {
            if wildcard {
                match &self.0[i] {
                    RoutePart::Wildcard => wildcard_vec.push(part.clone()),
                    RoutePart::Param(_) => {
                        return Err(Error::new(
                            "params may not immediately follow wildcards due to ambiguity",
                        ));
                    }
                    RoutePart::PathComponent(p) => {
                        if p == &part {
                            wildcard = false;
                            i += 1;
                            params.insert("*".to_string(), wildcard_vec.join("/"));
                        } else {
                            wildcard_vec.push(part.clone())
                        }
                    }
                    RoutePart::Leader => {
                        return Err(Error::new(
                            "Leaders may not follow wildcards. How'd you get here? :)",
                        ))
                    }
                }
            } else {
                match &self.0[i] {
                    RoutePart::Wildcard => {
                        wildcard_vec.push(part.clone());
                        wildcard = true;
                    }
                    RoutePart::Param(p) => {
                        params.insert(p.clone(), part.clone());
                    }
                    RoutePart::PathComponent(path_part) => {
                        if &part != path_part {
                            return Err(Error::new("invalid path for parameter extraction"));
                        }
                    }
                    RoutePart::Leader => {}
                };

                if self.0.len() - 1 > i {
                    i += 1;
                }
            }
        }

        if wildcard {
            params.insert("*".to_string(), wildcard_vec.join("/"));
        }

        Ok(params)
    }

    pub(crate) fn matches(&self, s: String) -> Result<bool, Error> {
        Ok(self.eq(&Self::new(s)?))
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        if other.0.len() != self.0.len() && !self.0.contains(&RoutePart::Wildcard) {
            return false;
        }

        let mut i = 0;
        let mut leader_seen = false;
        let mut wildcard = false;

        for arg in other.0.clone() {
            let res = match self.0[i].clone() {
                RoutePart::PathComponent(_) => self.0[i] == arg,
                RoutePart::Wildcard => {
                    if wildcard {
                        if self.0.len() < i + 1 {
                            let next = &self.0[i + 1];

                            match next {
                                RoutePart::PathComponent(_) => {
                                    if next == &arg {
                                        i += 1; // it will be incremented twice due to wildcard == false
                                        wildcard = false;
                                    }
                                }
                                _ => {}
                            };
                        }
                    } else {
                        wildcard = true
                    }

                    true
                }
                RoutePart::Param(_param) => {
                    // FIXME advanced parameter shit here later
                    true
                }
                RoutePart::Leader => {
                    if leader_seen {
                        false
                    } else {
                        leader_seen = true;
                        true
                    }
                }
            };

            if !res {
                return false;
            }

            if wildcard == false {
                i += 1;
            }
        }

        true
    }
}

impl Default for Path {
    fn default() -> Self {
        Self(vec![RoutePart::Leader])
    }
}

impl ToString for Path {
    fn to_string(&self) -> String {
        let mut s = Vec::new();

        for part in self.0.clone() {
            s.push(match part {
                RoutePart::Wildcard => "*".to_string(),
                RoutePart::PathComponent(pc) => pc.to_string(),
                RoutePart::Param(param) => {
                    format!(":{}", param)
                }
                RoutePart::Leader => "".to_string(),
            });
        }

        if s.len() < 2 {
            return "/".to_string();
        }

        s.join("/")
    }
}

mod tests {
    #[test]
    fn test_path() {
        use super::Path;
        use crate::Params;
        use std::collections::BTreeMap;

        let path = Path::new("/abc/def/ghi".to_string()).unwrap();
        assert!(path.matches("/abc/def/ghi".to_string()).unwrap());
        assert!(path.matches("//abc/def/ghi".to_string()).unwrap());
        assert!(!path.matches("/def/ghi".to_string()).unwrap());
        assert!(path.params().is_empty());

        let path = Path::new("/abc/:def/:ghi/jkl".to_string()).unwrap();
        assert!(!path.matches("/abc/def/ghi".to_string()).unwrap());
        assert!(path.matches("/abc/def/ghi/jkl".to_string()).unwrap());
        assert!(path.matches("/abc/ghi/def/jkl".to_string()).unwrap());
        assert!(path.matches("/abc/wooble/wakka/jkl".to_string()).unwrap());
        assert!(!path.matches("/nope/ghi/def/jkl".to_string()).unwrap());
        assert!(!path.matches("/abc/ghi/def/nope".to_string()).unwrap());
        assert_eq!(path.params().len(), 2);

        let mut bt = BTreeMap::new();
        bt.insert("def".to_string(), "wooble".to_string());
        bt.insert("ghi".to_string(), "wakka".to_string());

        assert_eq!(
            path.extract("/abc/wooble/wakka/jkl".to_string()).unwrap(),
            bt
        );
        assert!(path.extract("/wooble/wakka/jkl".to_string()).is_err());
        assert!(path.extract("/def/wooble/wakka/jkl".to_string()).is_err());

        assert_eq!(
            Path::new("/abc/:wooble/:wakka/jkl".to_string())
                .unwrap()
                .to_string(),
            "/abc/:wooble/:wakka/jkl".to_string()
        );

        assert_eq!(
            Path::new("/".to_string())
                .unwrap()
                .extract("/".to_string())
                .unwrap(),
            Params::default()
        );

        assert_eq!(
            Path::new("/account/".to_string()).unwrap(),
            Path::new("/account".to_string()).unwrap()
        );

        assert_eq!(Path::default().to_string(), "/".to_string());

        let path = Path::new("/".to_string()).unwrap();
        assert!(path.matches("/".to_string()).unwrap());

        assert!(Path::new("/abc/*/*".to_string()).is_err());
        assert!(Path::new("/abc/*/:param".to_string()).is_err());
        assert!(Path::new("/abc/*/a/b/c".to_string()).is_ok());

        let path = Path::new("/a/b/c".to_string()).unwrap();
        assert!(!path.matches("/a".to_string()).unwrap());

        let path = Path::new("/abc/*/a".to_string()).unwrap();
        assert!(path.matches("/abc/*/a".to_string()).unwrap());

        let mut p = Params::new();
        p.insert("*".to_string(), "foo/bar".to_string());
        assert_eq!(path.extract("/abc/foo/bar/a".to_string()).unwrap(), p);

        let path = Path::new("/abc/*/a/:test".to_string()).unwrap();
        let mut p = Params::new();
        p.insert("*".to_string(), "foo/bar".to_string());
        p.insert("test".to_string(), "quux".to_string());
        assert_eq!(path.extract("/abc/foo/bar/a/quux".to_string()).unwrap(), p);

        let path = Path::new("/wildcard/*".to_string()).unwrap();
        let mut p = Params::new();
        p.insert("*".to_string(), "frobnik/from/zorbo".to_string());
        assert_eq!(
            path.extract("/wildcard/frobnik/from/zorbo".to_string())
                .unwrap(),
            p
        )
    }
}
