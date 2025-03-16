use sailfish::Template;

#[derive(Template)]
#[template(path = "hello.stpl", escape = false)]
struct HelloTemplate {
    messages: Vec<String>,
    name: String,
}
pub trait Tpl {
    fn render(&self, name: String) -> String;
}

#[derive(Clone)]
pub struct TplRenderer {
    pub messages: Vec<String>,
}

impl Tpl for TplRenderer {
    fn render(&self, name: String) -> String {
        let ctx = HelloTemplate {
            messages: self.messages.clone(),
            name,
        };
        ctx.render().unwrap()
    }
}
