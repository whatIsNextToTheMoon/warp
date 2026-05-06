use pathfinder_geometry::vector::vec2f;
use warpui::elements::{
    Align, CacheOption, Clipped, ConstrainedBox, Container, CrossAxisAlignment, Flex, Image,
    MainAxisSize, ParentElement, Shrinkable, SizeConstraintCondition, SizeConstraintSwitch, Stack,
};
use warpui::Element;

pub const ONBOARDING_BG_PATH: &str = "async/png/onboarding/onboarding_bg.png";

const LEFT_COLUMN_WIDTH: f32 = 580.;
const LEFT_COLUMN_CONTENT_MAX_WIDTH: f32 = 800.;
const MIN_RIGHT_COLUMN_WIDTH: f32 = 540.;
pub const TWO_COLUMN_MIN_WIDTH: f32 = LEFT_COLUMN_WIDTH + MIN_RIGHT_COLUMN_WIDTH;

#[derive(Clone, Copy)]
pub struct ForegroundLayout;

pub const FOREGROUND_LAYOUT_DEFAULT: ForegroundLayout = ForegroundLayout;

pub fn static_left(
    left: impl Fn() -> Box<dyn Element>,
    right: impl Fn() -> Box<dyn Element>,
) -> Box<dyn Element> {
    let max_width_for_two_columns = LEFT_COLUMN_WIDTH + MIN_RIGHT_COLUMN_WIDTH;

    let left_constrained = || {
        ConstrainedBox::new(left())
            .with_max_width(LEFT_COLUMN_CONTENT_MAX_WIDTH)
            .finish()
    };

    let left_only = Align::new(left_constrained()).finish();
    let left_centered = Align::new(left_constrained()).finish();

    let left_fixed_width = Container::new(
        ConstrainedBox::new(left_centered)
            .with_width(LEFT_COLUMN_WIDTH)
            .finish(),
    )
    .finish();

    let right_flexible = Shrinkable::new(1., Container::new(right()).finish()).finish();

    let two_column_layout = Container::new(
        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(left_fixed_width)
            .with_child(right_flexible)
            .finish(),
    )
    .finish();

    SizeConstraintSwitch::new(
        two_column_layout,
        vec![(
            SizeConstraintCondition::WidthLessThan(max_width_for_two_columns),
            left_only,
        )],
    )
    .finish()
}

pub fn onboarding_right_panel_with_bg(
    path: &'static str,
    _layout: ForegroundLayout,
) -> Box<dyn Element> {
    let background = Image::new(ONBOARDING_BG_PATH, CacheOption::Original)
        .cover()
        .finish();

    let foreground = Container::new(
        Image::new(path, CacheOption::Original)
            .cover()
            .top_aligned()
            .finish(),
    )
    .with_padding_top(32.)
    .with_padding_left(39.)
    .with_padding_right(39.)
    .finish();

    let mut stack = Stack::new();
    stack.add_child(background);
    stack.add_child(foreground);

    Clipped::new(stack.finish()).finish()
}
