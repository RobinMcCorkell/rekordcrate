// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Rendering of Rekordbox waveform data to SVG.

use std::io::Write;

use svg::node::element::path::Data;
use svg::node::element::{Path, Rectangle, Text};
use svg::Document;

use crate::anlz::{Content, WaveformPreviewColumn, ANLZ};
use crate::Result;

/// Renders supported ANLZ waveform sections to SVG.
#[derive(Debug, Clone)]
pub struct WaveformRenderer {
    /// Height of each waveform plot in SVG units.
    pub height: u32,
    /// Fill color for monochrome waveforms.
    pub color: String,
    /// Background color for the SVG document.
    pub background: String,
}

impl Default for WaveformRenderer {
    fn default() -> Self {
        Self {
            height: 144,
            color: String::from("#2563eb"),
            background: String::from("#05070c"),
        }
    }
}

#[derive(Debug)]
struct Plot {
    label: &'static str,
    heights: Vec<u16>,
    maximum: u16,
    colors: Option<Vec<String>>,
    bands: Option<(Vec<u16>, Vec<u16>, Vec<u16>)>,
    bottom_aligned: bool,
}

impl WaveformRenderer {
    /// Creates a renderer with default appearance settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders all supported waveform previews in an ANLZ file to an SVG document.
    pub fn render_anlz(&self, anlz: &ANLZ) -> Result<Document> {
        let anlzs = [anlz];
        self.render_anlzs(&anlzs)
    }

    /// Renders all supported waveform previews from multiple ANLZ files to one SVG document.
    pub fn render_anlzs(&self, anlzs: &[&ANLZ]) -> Result<Document> {
        let plots = anlzs
            .iter()
            .flat_map(|anlz| anlz.sections.iter())
            .filter_map(|section| match &section.content {
                Content::WaveformPreview(preview) => Some(Plot {
                    label: "PWAV",
                    heights: preview
                        .data
                        .iter()
                        .map(|column| u16::from(column.height()))
                        .collect(),
                    maximum: 31,
                    colors: Some(
                        preview
                            .data
                            .iter()
                            .map(|column| blue_color(column.whiteness()))
                            .collect(),
                    ),
                    bands: None,
                    bottom_aligned: true,
                }),
                Content::WaveformBluePreview(preview) => Some(Plot {
                    label: "PWV2",
                    heights: preview
                        .data
                        .iter()
                        .map(|column| u16::from(column.height()))
                        .collect(),
                    maximum: 15,
                    colors: None,
                    bands: None,
                    bottom_aligned: true,
                }),
                Content::WaveformRGBPreview(preview) => Some(Plot {
                    label: "PWV4",
                    heights: preview
                        .data
                        .iter()
                        .map(|column| {
                            u16::from(column.energy_bottom_third_freq)
                                + u16::from(column.energy_mid_third_freq)
                                + u16::from(column.energy_top_third_freq)
                        })
                        .collect(),
                    maximum: preview
                        .data
                        .iter()
                        .map(|column| {
                            u16::from(column.energy_bottom_third_freq)
                                + u16::from(column.energy_mid_third_freq)
                                + u16::from(column.energy_top_third_freq)
                        })
                        .max()
                        .unwrap_or(1),
                    colors: Some(
                        preview
                            .data
                            .iter()
                            .map(|column| {
                                rgb_color(
                                    column.energy_bottom_third_freq,
                                    column.energy_mid_third_freq,
                                    column.energy_top_third_freq,
                                )
                            })
                            .collect(),
                    ),
                    bands: None,
                    bottom_aligned: true,
                }),
                Content::Waveform3BandPreview(preview) => Some(Plot {
                    label: "PWV6",
                    heights: preview
                        .data
                        .iter()
                        .map(|column| {
                            u16::from(column.energy_bottom_third_freq)
                                + u16::from(column.energy_mid_third_freq)
                                + u16::from(column.energy_top_third_freq)
                        })
                        .collect(),
                    maximum: preview
                        .data
                        .iter()
                        .map(|column| {
                            u16::from(column.energy_bottom_third_freq)
                                + u16::from(column.energy_mid_third_freq)
                                + u16::from(column.energy_top_third_freq)
                        })
                        .max()
                        .unwrap_or(1),
                    colors: None,
                    bands: Some((
                        preview
                            .data
                            .iter()
                            .map(|column| u16::from(column.energy_bottom_third_freq))
                            .collect(),
                        preview
                            .data
                            .iter()
                            .map(|column| u16::from(column.energy_mid_third_freq))
                            .collect(),
                        preview
                            .data
                            .iter()
                            .map(|column| u16::from(column.energy_top_third_freq))
                            .collect(),
                    )),
                    bottom_aligned: true,
                }),
                _ => None,
            })
            .collect::<Vec<_>>();

        if plots.is_empty() {
            return Err(
                std::io::Error::other("no supported waveform preview section found").into(),
            );
        }
        self.render_plots(&plots)
    }

    /// Renders monochrome waveform columns to an SVG document.
    pub fn render_columns(&self, columns: &[WaveformPreviewColumn]) -> Result<Document> {
        if columns.is_empty() {
            return Err(std::io::Error::other("waveform preview contains no columns").into());
        }
        self.render_plots(&[Plot {
            label: "PWAV",
            heights: columns
                .iter()
                .map(|column| u16::from(column.height()))
                .collect(),
            maximum: 31,
            colors: None,
            bands: None,
            bottom_aligned: false,
        }])
    }

    fn render_plots(&self, plots: &[Plot]) -> Result<Document> {
        let height = self.height.max(1);
        let width = plots
            .iter()
            .map(|plot| plot.heights.len())
            .max()
            .unwrap_or(0);
        let width = u32::try_from(width)
            .map_err(|_| std::io::Error::other("waveform preview contains too many columns"))?;
        if width == 0 {
            return Err(std::io::Error::other("waveform preview contains no columns").into());
        }
        let label_width = 42;
        let plot_width = width + label_width;
        let total_height = height
            .checked_mul(u32::try_from(plots.len()).unwrap_or(0))
            .ok_or_else(|| std::io::Error::other("waveform document is too tall"))?;

        let mut document = Document::new()
            .set("xmlns", "http://www.w3.org/2000/svg")
            .set("width", plot_width)
            .set("height", total_height)
            .set("viewBox", (0, 0, plot_width, total_height))
            .set("role", "img")
            .add(
                Rectangle::new()
                    .set("width", plot_width)
                    .set("height", total_height)
                    .set("fill", self.background.clone()),
            );

        for (plot_index, plot) in plots.iter().enumerate() {
            let top = height * u32::try_from(plot_index).unwrap_or(0);
            document = document
                .add(
                    Text::new(plot.label)
                        .set("x", 4)
                        .set("y", top + height / 2 + 5)
                        .set("fill", "#d1d5db")
                        .set("font-family", "monospace")
                        .set("font-size", 12),
                )
                .add(self.plot_element(plot, label_width, top, width, height));
        }

        Ok(document)
    }

    fn plot_element(
        &self,
        plot: &Plot,
        left: u32,
        top: u32,
        width: u32,
        height: u32,
    ) -> Box<dyn svg::node::Node> {
        if let Some((low, mid, high)) = &plot.bands {
            let divisor = plot.heights.len().max(1) as f32;
            let mut group = svg::node::element::Group::new();
            for column in 0..plot.heights.len() {
                let x = left as f32 + width as f32 * column as f32 / divisor;
                let mut lower = 0.0_f32;
                for (index, (value, color)) in
                    [(low, "#2563eb"), (mid, "#fde047"), (high, "#f8fafc")]
                        .into_iter()
                        .enumerate()
                {
                    let band_height =
                        f32::from(value[column]) / f32::from(plot.maximum.max(1)) * height as f32;
                    group = group.add(
                        Rectangle::new()
                            .set("x", x)
                            .set("y", top as f32 + height as f32 - lower - band_height)
                            .set("width", (width as f32 / divisor).max(1.0))
                            .set("height", band_height)
                            .set("fill", color)
                            .set("fill-opacity", [1.0, 0.64, 0.78][index]),
                    );
                    lower += band_height;
                }
            }
            return Box::new(group);
        }

        if let Some(colors) = &plot.colors {
            let divisor = plot.heights.len().max(1) as f32;
            let center = if plot.bottom_aligned {
                top as f32 + height as f32
            } else {
                top as f32 + height as f32 / 2.0
            };
            let mut group = svg::node::element::Group::new();
            for (index, (&value, color)) in plot.heights.iter().zip(colors).enumerate() {
                let x = left as f32 + width as f32 * index as f32 / divisor;
                let bar_height = f32::from(value) / f32::from(plot.maximum.max(1)) * height as f32;
                group = group.add(
                    Rectangle::new()
                        .set("x", x)
                        .set(
                            "y",
                            if plot.bottom_aligned {
                                center - bar_height
                            } else {
                                center - bar_height / 2.0
                            },
                        )
                        .set("width", (width as f32 / divisor).max(1.0))
                        .set("height", bar_height)
                        .set("fill", color.clone()),
                );
            }
            return Box::new(group);
        }

        let center = top as f32 + height as f32 / 2.0;
        let half_height = height as f32 / 2.0;
        let divisor = plot.heights.len().saturating_sub(1).max(1) as f32;
        let baseline = top as f32 + height as f32;
        let mut data = if plot.bottom_aligned {
            Data::new().move_to((left, baseline))
        } else {
            Data::new().move_to((left, center))
        };
        for (index, value) in plot.heights.iter().enumerate() {
            let x = left as f32 + width as f32 * index as f32 / divisor;
            let y = center - (f32::from(*value) / f32::from(plot.maximum.max(1))) * half_height;
            data = data.line_to((x, y));
        }
        for (index, value) in plot.heights.iter().enumerate().rev() {
            let x = left as f32 + width as f32 * index as f32 / divisor;
            let y = center + (f32::from(*value) / f32::from(plot.maximum.max(1))) * half_height;
            data = data.line_to((x, y));
        }
        let path = Path::new()
            .set("d", data.close())
            .set("fill", self.color.clone())
            .set("fill-opacity", 0.9);
        Box::new(path)
    }

    /// Renders an ANLZ file and writes the SVG document to a writer.
    pub fn render_to<W: Write>(&self, anlz: &ANLZ, writer: W) -> Result<()> {
        let anlzs = [anlz];
        self.render_anlzs_to(&anlzs, writer)
    }

    /// Renders multiple ANLZ files and writes one SVG document to a writer.
    pub fn render_anlzs_to<W: Write>(&self, anlzs: &[&ANLZ], writer: W) -> Result<()> {
        let document = self.render_anlzs(anlzs)?;
        svg::write(writer, &document)?;
        Ok(())
    }
}

fn blue_color(whiteness: u8) -> String {
    let brighten = |value: u8| {
        value.saturating_add(
            ((u16::from(u8::MAX - value) * u16::from(whiteness.min(7)) + 3) / 7) as u8,
        )
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        brighten(0x25),
        brighten(0x63),
        brighten(0xeb)
    )
}

fn rgb_color(low: u8, mid: u8, high: u8) -> String {
    // PWV4 stores energy, not display RGB. Normalize each channel by the column's total so
    // brightness is represented by the column height rather than lost in a dark raw color.
    let total = u16::from(low) + u16::from(mid) + u16::from(high);
    if total == 0 {
        return String::from("#000000");
    }
    let channel = |value: u8| ((u16::from(value) * 255 + total / 2) / total) as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(high),
        channel(mid),
        channel(low)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_columns_as_svg() {
        let columns = [
            WaveformPreviewColumn::new().with_height(0),
            WaveformPreviewColumn::new().with_height(31),
            WaveformPreviewColumn::new().with_height(0),
        ];
        let document = WaveformRenderer::default()
            .render_columns(&columns)
            .expect("columns should render");
        let svg = document.to_string();
        assert!(svg.contains("viewBox=\"0 0 45 144\""));
        assert!(svg.contains("<path"));
        assert!(svg.contains("#2563eb"));
    }
}
