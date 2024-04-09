use diesel::{
    deserialize::FromSql,
    mysql::{Mysql, MysqlValue},
    serialize::ToSql,
    sql_types,
};
use image::{io::Reader as ImageReader, DynamicImage, GenericImageView, ImageError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io::Cursor;

#[derive(FromSqlRow, AsExpression, Debug, PartialEq, Clone)]
#[diesel(sql_type = sql_types::VarChar)]
pub enum ImageMime {
    Jpeg,
    Png,
    Gif,
    Webp,
    Svg,
    Ai,
}

impl ImageMime {
    pub fn as_str(&self) -> &str {
        match *self {
            ImageMime::Jpeg => "image/jpeg",
            ImageMime::Png => "image/png",
            ImageMime::Gif => "image/gif",
            ImageMime::Webp => "image/webp",
            ImageMime::Svg => "image/svg+xml",
            ImageMime::Ai => "application/pdf",
        }
    }
    pub fn from_str(s: &str) -> Option<ImageMime> {
        match s {
            "image/jpeg" => Some(ImageMime::Jpeg),
            "image/png" => Some(ImageMime::Png),
            "image/gif" => Some(ImageMime::Gif),
            "image/webp" => Some(ImageMime::Webp),
            "image/svg+xml" => Some(ImageMime::Svg),
            "application/pdf" => Some(ImageMime::Ai),
            _ => None,
        }
    }
}

impl Serialize for ImageMime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ImageMime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ImageMime::from_str(&s).ok_or_else(|| serde::de::Error::custom("Invalid image format"))
    }
}

impl FromSql<sql_types::VarChar, Mysql> for ImageMime {
    fn from_sql(value: MysqlValue) -> diesel::deserialize::Result<Self> {
        let s = <String as FromSql<sql_types::VarChar, Mysql>>::from_sql(value)?;
        ImageMime::from_str(&s).ok_or_else(|| "Invalid image format".into())
    }
}

impl ToSql<sql_types::VarChar, Mysql> for ImageMime {
    fn to_sql<'a>(
        &'a self,
        out: &mut diesel::serialize::Output<'a, '_, Mysql>,
    ) -> diesel::serialize::Result {
        ToSql::<sql_types::VarChar, Mysql>::to_sql(self.as_str(), out)
    }
}

pub fn get_raster_image(
    bytes: &[u8],
) -> Result<(Option<ImageMime>, (u32, u32), DynamicImage), ImageError> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mime = match &reader.format() {
        Some(format) => match format {
            image::ImageFormat::Jpeg => Some(ImageMime::Jpeg),
            image::ImageFormat::Png => Some(ImageMime::Png),
            image::ImageFormat::Gif => Some(ImageMime::Gif),
            image::ImageFormat::WebP => Some(ImageMime::Webp),
            _ => None,
        },
        None => None,
    };
    let image = reader.decode()?;
    let dimensions = image.dimensions();
    Ok((mime, dimensions, image))
}
