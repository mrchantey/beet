use crate::render::Backend;




impl<T> Backend for T where T: ratatui::backend::Backend {}
