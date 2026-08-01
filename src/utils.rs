// Consolas glyph advance width relative to font-size
pub const CHAR_WIDTH_RATIO: f32 = 0.5498;

// boxes[cell_y][cell_x] vector boxes to hold every source-pixel triplet that falls into
// that character cell.
pub fn build_boxes(
    all_triplets: &Vec<(u8, u8, u8)>,
    total_width: usize,
    total_height: usize,
    cols: usize,
) -> Vec<Vec<Vec<(u8, u8, u8)>>> {
    // Rows is cols (80) * char-ratio * total height   all divided by  the total width
    // Given the above, and the ratio, how many rows will there be
    let total_rows = ((cols as f32) * CHAR_WIDTH_RATIO * (total_height as f32)
        / (total_width as f32))
        .round() as usize;

    // Nested vector generics, multiplied out by the cols and rows
    let mut boxes: Vec<Vec<Vec<(u8, u8, u8)>>> = vec![vec![Vec::new(); cols]; total_rows];

    // For each triplet (in bytes), push into into a box of the above predefined size
    for (i, &triplet) in all_triplets.iter().enumerate() {
        // how far into the current row am I?
        let x = i % total_width;
        // how many rows have I completed?
        let y = i / total_width;
        // scale (x, y) into cell coordinates without letting rounding drift
        // accumulate across the image - same reasoning as computing bin
        // boundaries per-index instead of via a fixed box_width.
        let cell_x = (x * cols) / total_width;
        let cell_y = (y * total_rows) / total_height;
        boxes[cell_y][cell_x].push(triplet);
    }

    boxes
}

pub fn pick_colors(boxes: &Vec<Vec<Vec<(u8, u8, u8)>>>) -> Vec<Vec<(u8, char)>> {
    // Declare colors as a generic of a Vec of Vec of us integer and char (color and character), for each cell
    let mut colors: Vec<Vec<(u8, char)>> = vec![vec![(0, ' '); boxes[0].len()]; boxes.len()];

    for (cell_y, row) in boxes.iter().enumerate() {
        for (cell_x, cell) in row.iter().enumerate() {
            // cell: &Vec<(u8, u8, u8)> - every source pixel that landed in this character cell
            
            // println!("cell ({}, {}) rgb values: {:?}", cell_x, cell_y, cell);

            // average cell's triplets into one (r, g, b)
            let mut final_box_color: (u32, u32, u32) = (0, 0, 0);
            
            // Walk every pixel in this cell one at a time, unpacking its (r, g, b)
            // bytes into their own variables, and add each into its matching
            // running total below - r's go with r's, g's with g's, b's with b's.
            for &(r, g, b) in cell {
                final_box_color.0 += r as u32;
                final_box_color.1 += g as u32;
                final_box_color.2 += b as u32;
            }
            let count = cell.len() as u32;
            let average_color = (
                (final_box_color.0 / count) as u8,
                (final_box_color.1 / count) as u8,
                (final_box_color.2 / count) as u8,
            );
            println!("cell ({}, {}) average color: {:?}", cell_x, cell_y, average_color);

            // quantize that average color into the xterm-256 palette.
            // xterm's 256-color palette only allows 6 possible brightness
            // levels per channel (0-5), so first work out which of those 6
            // levels each channel is closest to, then combine the three
            // levels into one final palette index.
            let (avg_r, avg_g, avg_b) = average_color;

            // 255 / 51 = 5, so dividing a 0-255 channel by 51 rounds it down
            // into one of the 6 allowed levels (0-5).
            let r_step = avg_r / 51;
            let g_step = avg_g / 51;
            let b_step = avg_b / 51;

            // The palette is a 6x6x6 cube starting at index 16 - every
            // unique (r_step, g_step, b_step) combination maps to exactly
            // one index, the same way you'd flatten a 3D array into a 1D one.
            let palette_index: u8 = 16 + (r_step * 36) + (g_step * 6) + b_step;
            println!("cell ({}, {}) palette index: {}", cell_x, cell_y, palette_index);

            // pick an ascii glyph based on the cell's average luminance.
            // "luminance" here just means overall brightness - how light or
            // dark the averaged color is. Treat r, g, b as equally
            // important and just average all three together.
            let luminance = (avg_r as u32 + avg_g as u32 + avg_b as u32) / 3;

            // glyphs ordered from sparse-looking (space) to dense-looking
            // (#) - darker cells should get sparser-looking characters,
            // brighter cells should get denser-looking ones.
            let glyph_ramp = " `.,-:+*%$#";
            let glyph_count = glyph_ramp.chars().count();

            // luminance ranges 0-255. Split that range into glyph_count
            // equal-sized buckets, then figure out which bucket this
            // cell's luminance falls into - same bucketing idea as the
            // color-quantization step above, just with a different range
            // and a different number of buckets.
            let bucket_size = 256 / glyph_count;
            let mut glyph_index = luminance as usize / bucket_size;

            // luminance == 255 can round into an index one past the last
            // glyph - clamp it back onto the last valid character so we
            // never index out of bounds.
            if glyph_index >= glyph_count {
                glyph_index = glyph_count - 1;
            }

            let glyph = glyph_ramp.chars().nth(glyph_index).unwrap();
            println!("cell ({}, {}) glyph: {}", cell_x, cell_y, glyph);
            colors[cell_y][cell_x] = (palette_index, glyph);
        }
    }
    colors
}

// Recall the real .ans files look like (per cell, repeated across a row):
//   \x1b[38;5;16m\x1b[48;5;16m`
// then at the end of each row:
//   \x1b[0m\n
// (escape codes set fg/bg color, then one glyph character, repeated per
// cell; a reset code + newline ends the row.)
pub fn print_grid(colors: &Vec<Vec<(u8, char)>>, path: &str) {
    // builds up the entire grid, one row's worth of text at a time
    let mut output = String::new();

    // loop over colors row by row (colors.iter())
    for (cell_y, row) in colors.iter().enumerate() {
        // builds up just this one row's worth of text
        let mut row_output = String::new();

        //  for each row, loop over its cells (row.iter()) - each cell is
        //  already a (palette_index, glyph) pair from pick_colors, ready to use directly
        for (cell_x, cell) in row.iter().enumerate() {
            // tells the terminal which color to draw the character itself in
            let fg_escape = format!("\x1b[38;5;{}m", cell.0);

            // tells the terminal which color to fill the space behind the character with
            let bg_escape = format!("\x1b[48;5;{}m", cell.0);

            // sticks the actual visible character right after the two color codes
            let cell_output = format!("{}{}{}", fg_escape, bg_escape, cell.1);

            // add this cell's text onto the end of the row we're building
            row_output.push_str(&cell_output);
        }

        // row's cells are done - reset the colors and start a new line
        row_output.push_str("\x1b[0m\n");

        // add this finished row onto the end of the whole grid
        output.push_str(&row_output);
    }

    // show it in the terminal too, same as before
    print!("{}", output);

    // this is the actual deliverable - a real .ans file, same as the ones
    std::fs::write(path, &output).unwrap();
    println!("wrote ANSI output to {}", path);
}
