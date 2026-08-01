// image_or_video_to_ansi_frames
// CLI flags
use clap::Parser;
use std::fs::read;
use std::io::BufReader;
use zune_jpeg::JpegDecoder;

mod html_out;
mod utils;

#[derive(Parser)]
struct Args {
    /// Name of the file top convert
    #[arg(long, default_value = "./test_data/test.jpeg")]
    image_or_video_file: String,

    /// Directory to zip
    #[arg(long, default_value = "./test_data/output.ansi")]
    output_file_name: String,

    /// Also write a minimal standalone HTML file rendering the ANSI art,
    /// styled the same way claytons-philosophy-notes displays it
    #[arg(long)]
    create_html_output: bool,

    /// Where to write the HTML output, if --create-html-output is set
    #[arg(long, default_value = "./test_data/output.html")]
    html_output_file: String,
}

// Loose starting scaffold only - no logic here on purpose, this is yours to
// build. See ../proj_ideas_to_learn_rust.md for the full staged plan. What's
// below just sketches the *shape* of stage 1 (single image -> a grid of
// characters printed to stdout, no color, no video yet). Everything else
// (real xterm-256 color, video/gif frames, rayon parallelism) comes later,
// once this smallest version actually runs.
//
// Candidate crates for image decode (pick one yourself via `cargo add`,
// don't just grab both):
//   - zune-jpeg   lighter, more manual, probably the better learning pick
//   - image       batteries-included, hides more of the interesting part
//
// Crates you'll probably want in later stages, not yet:
//   - rayon       parallel iterators - for frame-level parallelism, once
//                 sequential frame-by-frame conversion already works
//   - gif         GIF frame decoding, once you get to the video/animation
//                 stage (see the "why video" section in the plan doc)

fn main() {
    // 1. Read CLI args: path to the image, target width in character
    //    columns, maybe a mode flag. std::env::args() gives you an
    //    iterator - decide how you want to validate arg count / bail out
    //    with a usage message if something's missing.
    let args = Args::parse();
    println!("Image or video file {}", args.image_or_video_file);
    let image_or_video_file = args.image_or_video_file;

    if image_or_video_file.contains(".jpeg") || image_or_video_file.contains(".jpg") {
        println!("Was .JPG or .JPEG")
    } else {
        println!("Was alternate file format...")
    }

    // 2. Open + decode the image into some kind of pixel buffer. Whatever
    //    crate you pick will hand you back width/height plus raw pixel
    //    bytes (or a typed image struct) - get comfortable with the shape
    //    of that data before doing anything else with it.
    let file_contents = BufReader::new(std::fs::File::open(image_or_video_file).unwrap());
    println!("file_contents: {:?}", file_contents);
    let mut decoder = JpegDecoder::new(file_contents);
    let mut pixels = decoder.decode().unwrap();
    println!("pixel count: {}", pixels.len());
    println!("first 20 bytes: {:?}", &pixels[..20]);
    let info = decoder.info().unwrap();
    println!(
        "Decoder info: width={} height={} components={}",
        info.width, info.height, info.components
    );
    // RGB output is the default DecoderOptions colorspace, so pixels is always 3 bytes/pixel here.
    let stride = 3;
    
    // 3. Work out the downsample: the source image almost certainly has
        // 3.1 Decode full resolution.
        // 3.2 For each character row i in 0..rows, pick source rows 2*i and 2*i+1 directly (no averaging) — this alone gets you correct-looking aspect ratio for free, matching the reference.
        // 3.3 For each character column j in 0..cols, average all source pixels in that column's bin (width/cols source columns wide) across both of those two rows — gives you two colors per cell (fg from top row's bin-average, bg from bottom row's bin-average).
        // 3.4 Quantize each to the 256-cube with the same linear /51-style rounding as rgb()/paletteColor(), so it's consistent with the existing .ans corpus and lib/ansi.js.
        // 3.5 Pick the ASCII glyph from the same 11-char luminance ramp using the two cells' combined average brightness, and always emit it (not a blank), to match the existing art's texture.
    
    // Break the pixel byte value RGB triplets into boxes the size of a character
    let all_triplets: Vec<(u8, u8, u8)> = pixels
        .chunks_exact(stride)
        .map(|c| (c[0], c[1], c[2]))
        .collect();
    
    println!("triplet count: {}", all_triplets.len());
    println!("first 5 triplets: {:?}", &all_triplets[..5]);

    let total_width = info.width as usize;
    let total_height = info.height as usize;
    let cols: usize = 80; // hardcoded for now - will become a CLI flag later

    let boxes = utils::build_boxes(&all_triplets, total_width, total_height, cols);

    println!("grid: {} cols x {} rows", cols, boxes.len());
    println!("pixels in cell (0,0): {}", boxes[0][0].len());

    // 4. do color somehow
    //   - real color: xterm-256 palette + nearest-color search per cell
    //     (lib/ansi.js's paletteColor() in claytons-philosophy-notes has
    //     the palette math in the reverse direction, as a reference)
    let colors = utils::pick_colors(&boxes);



    // 5. Print the grid, one row at a time, newline between rows. That's
    //    the whole first milestone - a real image in, a recognizable blob
    //    of characters out.
    utils::print_grid(&colors, &args.output_file_name);

    if args.create_html_output {
        html_out::write_html_output(&colors, cols, &args.html_output_file);
    }

    // --- once that works end to end, next things to sketch in here are: ---
    //   - emitting real ANSI escape codes instead of plain characters
    //   - video/gif: same per-frame conversion, called once per frame
    //   - parallelism: rayon's par_iter() across frames - only after
    //     sequential frame-by-frame conversion is already correct
}
