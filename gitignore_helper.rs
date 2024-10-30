use ignore::gitignore::{GitignoreBuilder, Gitignore};

pub struct GitignoreHelper;

impl GitignoreHelper {
    pub fn build() -> Option<Gitignore> {
        let mut builder = GitignoreBuilder::new(".");
        if builder.add(".gitignore").is_none() {
            builder.build().ok()
        } else {
            None
        }
    }
}
