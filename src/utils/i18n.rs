use rocket::{
    request::{self, FromRequest},
    Request,
};

pub struct I18n<'a> {
    pub locale: &'a str,
}

impl<'a> I18n<'a> {
    pub fn new(locale: &'a str) -> Self {
        Self { locale }
    }

    pub fn t(&self, key: &'a str) -> String {
        rust_i18n::t!(key, locale = self.locale).to_string()
    }

    pub fn tf(&self, key: &'a str, options: &[(&'a str, String)]) -> String {
        let message = rust_i18n::t!(key, locale = self.locale);
        let patterns = options.iter().map(|(k, _)| *k).collect::<Vec<_>>();
        let values = options.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>();
        return rust_i18n::replace_patterns(message.as_ref(), &patterns, &values);
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for I18n<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
        let user_locale = request.headers().get_one("X-USER-LOCALE");

        if let Some(locale) = user_locale {
            return request::Outcome::Success(I18n::new(locale));
        }

        let accept_locales = request.headers().get_one("Accept-Language");

        if let Some(locales) = accept_locales {
            let locales: Vec<&str> = locales.split(',').collect();

            for locale in locales {
                if let Some(locale) = locale.split(';').next() {
                    if locale.starts_with("en") {
                        return request::Outcome::Success(I18n::new("en"));
                    }
                    if locale.starts_with("zh") {
                        return request::Outcome::Success(I18n::new("zh"));
                    }
                }
            }
        }

        request::Outcome::Success(I18n::new("en"))
    }
}
