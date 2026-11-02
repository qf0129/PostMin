use eframe::egui::{
    self, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Margin, Rounding,
    Stroke, Style, TextStyle, Vec2,
};
use postmin_core::ThemeMode;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

pub const MONO: f32 = 12.5;

static MODE: AtomicU8 = AtomicU8::new(0);
static PPI_HUNDREDTHS: AtomicU32 = AtomicU32::new(100);

fn is_light() -> bool {
    MODE.load(Ordering::Relaxed) == 1
}

fn pixels_per_point() -> f32 {
    PPI_HUNDREDTHS.load(Ordering::Relaxed) as f32 / 100.0
}

/// Snap a logical font size to a whole physical pixel.
///
/// epaint rasterizes glyphs at integer pixel sizes but computes line metrics from
/// the unrounded size; fractional sizes therefore land on half-pixel baselines and
/// look soft. Snapping keeps raster size and layout metrics consistent.
pub fn font_size(size: f32) -> f32 {
    let ppp = pixels_per_point();
    (size * ppp).round() / ppp
}

fn pick(dark: [u8; 3], light: [u8; 3]) -> Color32 {
    let value = if is_light() { light } else { dark };
    Color32::from_rgb(value[0], value[1], value[2])
}

pub fn bg() -> Color32 {
    pick([17, 18, 22], [244, 245, 248])
}

pub fn panel() -> Color32 {
    pick([23, 24, 29], [250, 250, 252])
}

pub fn panel_2() -> Color32 {
    pick([28, 29, 35], [240, 241, 246])
}

pub fn card() -> Color32 {
    pick([33, 35, 42], [232, 234, 240])
}

pub fn input() -> Color32 {
    pick([19, 20, 25], [255, 255, 255])
}

pub fn hover() -> Color32 {
    pick([42, 44, 52], [222, 226, 233])
}

pub fn border() -> Color32 {
    pick([48, 50, 60], [208, 213, 222])
}

pub fn border_soft() -> Color32 {
    pick([39, 41, 49], [228, 231, 237])
}

pub fn text() -> Color32 {
    pick([226, 229, 235], [30, 34, 42])
}

pub fn text_dim() -> Color32 {
    pick([162, 166, 177], [72, 80, 95])
}

pub fn muted() -> Color32 {
    pick([143, 148, 161], [96, 104, 119])
}

pub fn accent() -> Color32 {
    pick([255, 122, 69], [214, 78, 32])
}

pub fn accent_soft() -> Color32 {
    pick([255, 165, 128], [237, 125, 84])
}

pub fn accent_dim() -> Color32 {
    pick([76, 45, 32], [250, 226, 215])
}

pub fn green() -> Color32 {
    pick([74, 222, 128], [22, 148, 68])
}

pub fn blue() -> Color32 {
    pick([96, 165, 250], [37, 99, 235])
}

pub fn purple() -> Color32 {
    pick([192, 132, 252], [124, 58, 237])
}

pub fn yellow() -> Color32 {
    pick([250, 204, 21], [174, 83, 10])
}

pub fn red() -> Color32 {
    pick([248, 113, 113], [206, 38, 38])
}

pub fn cyan() -> Color32 {
    pick([103, 232, 249], [8, 145, 178])
}

/// Semi-transparent tint of a color, suitable for both dark and light backgrounds.
pub fn tint(color: Color32, alpha: f32) -> Color32 {
    let alpha = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn install(ctx: &Context) {
    install_fonts(ctx);
    apply(ctx, ThemeMode::Light);
}

pub fn apply(ctx: &Context, mode: ThemeMode) {
    MODE.store(
        match mode {
            ThemeMode::Dark => 0,
            ThemeMode::Light => 1,
        },
        Ordering::Relaxed,
    );
    PPI_HUNDREDTHS.store(
        (ctx.pixels_per_point() * 100.0).round() as u32,
        Ordering::Relaxed,
    );
    let mut style = Style::default();
    let light = mode == ThemeMode::Light;
    let mut visuals = if light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };

    visuals.panel_fill = panel();
    visuals.window_fill = panel_2();
    visuals.extreme_bg_color = input();
    visuals.faint_bg_color = panel_2();
    visuals.code_bg_color = card();
    // Keep `override_text_color` unset: egui paints `TextEdit` hint text with
    // `weak_text_color()`, but an override would force hints to the normal text color.
    visuals.hyperlink_color = accent();
    visuals.window_rounding = Rounding::same(10.0);
    visuals.menu_rounding = Rounding::same(8.0);
    visuals.window_stroke = Stroke::new(1.0_f32, border());
    visuals.text_cursor = Stroke::new(2.0_f32, accent());
    visuals.selection.bg_fill = tint(accent(), if light { 0.24 } else { 0.32 });
    visuals.selection.stroke = Stroke::new(1.0_f32, accent_soft());
    visuals.window_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 10.0),
        blur: if light { 20.0 } else { 28.0 },
        spread: 0.0,
        color: Color32::from_black_alpha(if light { 48 } else { 150 }),
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 8.0),
        blur: if light { 16.0 } else { 22.0 },
        spread: 0.0,
        color: Color32::from_black_alpha(if light { 38 } else { 130 }),
    };
    visuals.striped = true;

    visuals.widgets.noninteractive.bg_fill = panel_2();
    visuals.widgets.noninteractive.weak_bg_fill = panel_2();
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border_soft());
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text());

    visuals.widgets.inactive.bg_fill = card();
    visuals.widgets.inactive.weak_bg_fill = card();
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border_soft());
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text());

    visuals.widgets.hovered.bg_fill = hover();
    visuals.widgets.hovered.weak_bg_fill = hover();
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, border());
    visuals.widgets.hovered.fg_stroke =
        Stroke::new(1.0_f32, if light { text() } else { Color32::WHITE });

    let active_fill = if light { accent() } else { accent_dim() };
    visuals.widgets.active.bg_fill = active_fill;
    visuals.widgets.active.weak_bg_fill = active_fill;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, accent());
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    visuals.widgets.open.bg_fill = card();
    visuals.widgets.open.weak_bg_fill = card();
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, border());
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, text());

    style.visuals = visuals;
    style.spacing.item_spacing = Vec2::new(8.0, 7.0);
    style.spacing.button_padding = Vec2::new(11.0, 5.0);
    style.spacing.interact_size = Vec2::new(40.0, 26.0);
    style.spacing.menu_margin = Margin::same(6.0);
    style.spacing.window_margin = Margin::same(12.0);
    style.spacing.indent = 16.0;

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(font_size(19.0), FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(font_size(13.0), FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(font_size(13.0), FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(font_size(11.5), FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(font_size(MONO), FontFamily::Monospace),
    );

    ctx.set_style(style);
}

fn install_fonts(ctx: &Context) {
    let candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\Deng.ttf",
        r"C:\Windows\Fonts\simhei.ttf",
        r"/mnt/c/Windows/Fonts/msyh.ttc",
        r"/mnt/c/Windows/Fonts/Deng.ttf",
        r"/mnt/c/Windows/Fonts/simhei.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/System/Library/Fonts/PingFang.ttc",
    ];

    let Some(bytes) = candidates.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };

    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("postmin-cjk".to_owned(), FontData::from_owned(bytes));
    // Use the CJK font as the primary proportional face so Chinese and Latin
    // share weight, baseline and metrics; keep it as a fallback for monospace.
    if let Some(entries) = fonts.families.get_mut(&FontFamily::Proportional) {
        entries.insert(0, "postmin-cjk".to_owned());
    }
    if let Some(entries) = fonts.families.get_mut(&FontFamily::Monospace) {
        entries.push("postmin-cjk".to_owned());
    }
    ctx.set_fonts(fonts);
}

pub fn primary_button(label: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(label.into())
            .color(Color32::WHITE)
            .strong()
            .size(font_size(13.0)),
    )
    .fill(accent())
    .rounding(Rounding::same(7.0))
    .min_size(Vec2::new(0.0, 32.0))
}

pub fn secondary_button(label: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(label.into())
            .color(text())
            .size(font_size(12.5)),
    )
    .fill(card())
    .stroke(Stroke::new(1.0_f32, border_soft()))
    .rounding(Rounding::same(6.0))
    .min_size(Vec2::new(0.0, 30.0))
}

pub fn ghost_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(text.into())
            .color(text_dim())
            .size(font_size(12.5)),
    )
    .frame(false)
    .min_size(Vec2::new(0.0, 26.0))
}

pub fn small_ghost(label: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(label.into())
            .color(muted())
            .size(font_size(11.5)),
    )
    .frame(false)
    .min_size(Vec2::new(0.0, 20.0))
}

/// Small accent action button used for `+ 添加`, `启用`, etc.
pub fn accent_small(label: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(label.into())
            .color(accent())
            .size(font_size(11.5)),
    )
    .fill(tint(accent(), 0.10))
    .stroke(Stroke::new(1.0_f32, tint(accent(), 0.28)))
    .rounding(Rounding::same(5.0))
    .min_size(Vec2::new(0.0, 22.0))
}
