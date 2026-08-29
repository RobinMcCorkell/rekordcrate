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
    heights: Vec<u8>,
    maximum: u8,
    colors: Option<Vec<String>>,
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
                    heights: preview.data.iter().map(|column| column.height()).collect(),
                    maximum: 31,
                    colors: None,
                }),
                Content::TinyWaveformPreview(preview) => Some(Plot {
                    label: "PWV2",
                    heights: preview.data.iter().map(|column| column.height()).collect(),
                    maximum: 15,
                    colors: None,
                }),
                Content::WaveformColorPreview(preview) => Some(Plot {
                    label: "PWV4",
                    heights: preview
                        .data
                        .iter()
                        .map(|column| {
                            column
                                .energy_bottom_third_freq
                                .max(column.energy_mid_third_freq)
                                .max(column.energy_top_third_freq)
                        })
                        .collect(),
                    maximum: u8::MAX,
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
                }),
                Content::Waveform3BandPreview(preview) => Some(Plot {
                    label: "PWV6",
                    heights: preview
                        .data
                        .iter()
                        .map(|column| {
                            column
                                .energy_bottom_third_freq
                                .max(column.energy_mid_third_freq)
                                .max(column.energy_top_third_freq)
                        })
                        .collect(),
                    maximum: u8::MAX,
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
            heights: columns.iter().map(|column| column.height()).collect(),
            maximum: 31,
            colors: None,
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
        if let Some(colors) = &plot.colors {
            let divisor = plot.heights.len().max(1) as f32;
            let center = top as f32 + height as f32 / 2.0;
            let mut group = svg::node::element::Group::new();
            for (index, (&value, color)) in plot.heights.iter().zip(colors).enumerate() {
                let x = left as f32 + width as f32 * index as f32 / divisor;
                let bar_height = f32::from(value) / f32::from(plot.maximum.max(1)) * height as f32;
                group = group.add(
                    Rectangle::new()
                        .set("x", x)
                        .set("y", center - bar_height / 2.0)
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
        let mut data = Data::new().move_to((left, center));
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

fn rgb_color(low: u8, mid: u8, high: u8) -> String {
    // The preview stores frequency energy rather than an explicit RGB color. Use the three
    // frequency bands as a simple, stable visualization: low=blue, mid=green, high=red.
    format!("#{high:02x}{mid:02x}{low:02x}")
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
