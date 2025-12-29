use iced::widget::canvas::{self, Cache, Canvas, Geometry, Path, Stroke, Frame};
use iced::{Color, Point, Size, Renderer, Theme, Rectangle};
use crate::stock::StockData;

pub struct Chart {
    data: StockData,
    cache: Cache,
}

impl Chart {
    pub fn new(data: StockData) -> Self {
        Self {
            data,
            cache: Cache::new(),
        }
    }
}

impl canvas::Program<()> for Chart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
             if self.data.history.len() < 2 {
                return;
            }

            let start_date = self.data.history.first().unwrap().0;
            let end_date = self.data.history.last().unwrap().0;
            
            let min_price = self.data.history.iter().map(|x| x.3).fold(f64::INFINITY, |a, b| a.min(b));
            let max_price = self.data.history.iter().map(|x| x.2).fold(f64::NEG_INFINITY, |a, b| a.max(b));

            let range_price = max_price - min_price;
            let padding_price = range_price * 0.1;
            let min_y = min_price - padding_price;
            let max_y = max_price + padding_price;
            let range_y = max_y - min_y;

            let width = bounds.width;
            let height = bounds.height;
            
            // Normalize and create path
            let points: Vec<Point> = self.data.history.iter().enumerate().map(|(i, item)| {
                let x = (i as f32 / (self.data.history.len() - 1) as f32) * width;
                // Y axis is usually top-down in graphics, so we flip
                let y = height - ((item.4 - min_y) / range_y * height as f64) as f32; 
                Point::new(x, y)
            }).collect();

            let path = Path::new(|builder| {
                builder.move_to(points[0]);
                for p in points.iter().skip(1) {
                    builder.line_to(*p);
                }
            });

            let color = if self.data.change_percent >= 0.0 {
                Color::from_rgb(0.0, 0.8, 0.0)
            } else {
                Color::from_rgb(0.8, 0.0, 0.0)
            };

            frame.stroke(&path, Stroke::default().with_color(color).with_width(2.0));
        });

        vec![geometry]
    }
}

pub fn view(data: &StockData) -> Canvas<Chart, ()> {
    Canvas::new(Chart::new(data.clone()))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
}
