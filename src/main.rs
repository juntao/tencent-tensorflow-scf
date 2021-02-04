use std::io::{self, Read};
use serde::Deserialize;
use image;
use image::{GenericImageView};
use imageproc::{drawing};
use rusttype::{Font, Scale};
mod imagecrop;

fn main() {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).expect("Error reading from STDIN");
    let obj: FaasInput = serde_json::from_str(&buffer).unwrap();
    // let split = &(obj.body).split(",");
    let body = obj.body.clone();
    let split = body.split(",");
    let vec: Vec<&str> = split.collect();

    let img_buf = base64::decode_config(vec[4], base64::STANDARD).unwrap();
    // println!("Image buf size is {}", img_buf.len());
    
    let result = write_greeting(vec[0], vec[1], vec[2], vec[3], &img_buf);
    println!("{}", base64::encode_config(result, base64::STANDARD));
}

#[derive(Deserialize, Debug)]
struct FaasInput {
    body: String
}

const COVER_IMG_DIM: u32 = 600; // User upload image will be resized to this dimension

const TOP_SCALE: Scale = Scale {
  x: 80.0,
  y: 80.0,
};
const GREETING_SCALE: Scale = Scale {
  x: 40.0,
  y: 40.0,
};
const SIGN_SCALE: Scale = Scale {
  x: 30.0,
  y: 30.0,
};

const TEXT_COLOR: [image::Rgba<u8>; 2] = [image::Rgba([211, 171, 145, 255]), image::Rgba([150, 77, 75, 255])];
const BG1_COLOR: [image::Rgba<u8>; 2] = [image::Rgba([150, 77, 75, 255]), image::Rgba([249, 239, 215, 255])];
const BG2_COLOR: [image::Rgba<u8>; 2] = [image::Rgba([175, 113, 113, 255]), image::Rgba([255, 250, 237, 255])];
const BG_BORDER: (u32, u32, u32) = (180, 50, 170);

const FONT_FILE : &[u8] = include_bytes!("PingFang Bold.ttf") as &[u8];
const QR_BUF : &[u8] = include_bytes!("qr.png") as &[u8];

fn write_to_crop(watermark_text: &str, scale: Scale) -> u32 {
  let buffer = include_bytes!("crop.png") as &[u8];
  let mut img = image::load_from_memory(&buffer.to_vec()).unwrap();

  let font = Vec::from(FONT_FILE);
  let font = Font::try_from_vec(font).unwrap();

  drawing::draw_text_mut(&mut img, image::Rgba([0, 0, 0, 255]), 0, 0, scale, &font, watermark_text);

  let ic = imagecrop::ImageCrop::new(img).unwrap();
  let corners = ic.calculate_corners();
  return corners.1.x - corners.0.x;
}

pub fn write_greeting(theme: &str, from: &str, greeting: &str, to: &str, img_buf: &[u8]) -> Vec<u8> {
  let theme = theme.parse::<usize>().unwrap();
  let font = Vec::from(FONT_FILE);
  let font = Font::try_from_vec(font).unwrap();

  let mut img = overlay_background(theme, img_buf);
  let img_width = img.width();
  let img_height = img.height();

  // Draw top text and underline
  let mut width = write_to_crop(to, TOP_SCALE);
  let mut scale = TOP_SCALE;
  let perc = COVER_IMG_DIM as f32 / width as f32;
  if width > COVER_IMG_DIM {
    scale = Scale {
      x: TOP_SCALE.x * perc,
      y: TOP_SCALE.x * perc,
    };
    width = COVER_IMG_DIM;
  }
  // Text height to be reduced
  let reduced_height = (70.0 * (1.0 - perc)) as u32;
  drawing::draw_text_mut(&mut img, TEXT_COLOR[theme], BG_BORDER.1, 10, TOP_SCALE, &font, "致：");
  drawing::draw_text_mut(&mut img, TEXT_COLOR[theme], BG_BORDER.1, 90, scale, &font, to);
  for y in 0..=4 {
    drawing::draw_line_segment_mut(
      &mut img,
      ((BG_BORDER.1) as f32, (85 + y) as f32),
      ((BG_BORDER.1 + 80) as f32, (85 + y) as f32),
      TEXT_COLOR[theme]
    );
    drawing::draw_line_segment_mut(
      &mut img,
      ((BG_BORDER.1) as f32, (165 - reduced_height + y) as f32),
      ((BG_BORDER.1 + width) as f32, (165 - reduced_height + y) as f32),
      TEXT_COLOR[theme]
    );
  }

  // Draw greeting text
  let greeting_vec = greeting.split("\n");
  let greeting_vec = greeting_vec.collect::<Vec<&str>>();
  let mut c = 0;
  for (i, &g) in greeting_vec.iter().enumerate() {
    let width = write_to_crop(g, GREETING_SCALE);
    drawing::draw_text_mut(
      &mut img,
      TEXT_COLOR[theme],
      img_width - BG_BORDER.1 - width,
      img_height - BG_BORDER.2 + 10 + (i as u32 * 40),
      GREETING_SCALE,
      &font, g);
    c = i + 1;
  }

  // Draw from text
  let from = "—— ".to_owned() + from;
  let width = write_to_crop(&from, SIGN_SCALE);
  drawing::draw_text_mut(
    &mut img,
    TEXT_COLOR[theme],
    img_width - BG_BORDER.1 - width,
    img_height - BG_BORDER.2 + 30 + (c as u32 * 40),
    SIGN_SCALE,
    &font, &from);

  let mut buf = vec![];
  img.write_to(&mut buf, image::ImageOutputFormat::Png).unwrap();
  return buf;
}

fn overlay_background(theme: usize, img_buf: &[u8]) -> image::DynamicImage {
  let img = image::load_from_memory(img_buf).unwrap();
  let mut cover_dim = COVER_IMG_DIM;
  // Preserve the origin image's aspect ratio
  if img.width() < img.height() {
    cover_dim = ((img.height() as f32 / img.width() as f32) * cover_dim as f32) as u32;
  }
  let img = img.resize(cover_dim, cover_dim, image::imageops::FilterType::Triangle);

  let width = img.width() + 2 * BG_BORDER.1;
  let height = img.height() + BG_BORDER.0 + BG_BORDER.2;
  let back = image::ImageBuffer::from_fn(
    width,
    height,
    |_x, y| {
      if y < height / 2 {
        BG1_COLOR[theme]
      } else {
        BG2_COLOR[theme]
      }
    }
  );
  let mut back = image::DynamicImage::ImageRgba8(back);

  image::imageops::overlay(&mut back, &img, BG_BORDER.1, BG_BORDER.0);

  let qr = image::load_from_memory(QR_BUF).unwrap();
  image::imageops::overlay(&mut back, &qr, BG_BORDER.1, BG_BORDER.0 + &img.height() + 20);

  return back;
}
