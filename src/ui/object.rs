#[derive(Debug, Clone)]
pub enum RenderObject {
    Text {
        text: String,
        x: f64,
        y: f64,
        size: f64,
        color: (f64, f64, f64),
    },
    Rectangle {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: (f64, f64, f64),
    },
}
