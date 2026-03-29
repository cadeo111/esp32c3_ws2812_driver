use crate::strip_based::Color24bit;

pub trait LedGrid<
    const HEIGHT: usize,
    const WIDTH: usize,
    const DEPTH: usize,
    const TOTAL: usize,
    GridItem: Copy + Sized + Color24bit,
    GridType: Grid<HEIGHT, WIDTH, GridItem>
>
{
  

    fn clear_all(&mut self);
    fn get_grid_mut(&mut self, z: usize) -> &mut GridType;
}

pub trait Grid<const HEIGHT: usize, const WIDTH: usize, GridItem: Copy + Sized>
{
    fn get_at_mut(&mut self, x: usize, y: usize) -> &mut GridItem;

    fn apply_to_all<M: Fn(&mut GridItem), F: Fn(&mut GridItem) -> bool>(
        &mut self,
        modifier: M,
        filter: F,
    ) {
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                let item = self.get_at_mut(x, y);
                if filter(item) {
                    modifier(item);
                }
            }
        }
    }

    fn apply_to_x<M: Fn(&mut GridItem), F: Fn(&mut GridItem) -> bool>(
        &mut self,
        x: usize,
        modifier: M,
        filter: F,
    ) {
        for y in 0..HEIGHT {
            let item = self.get_at_mut(x, y);
            if filter(item) {
                modifier(item);
            }
        }
    }

    fn apply_to_y<M: Fn(&mut GridItem), F: Fn(&mut GridItem) -> bool>(
        &mut self,
        y: usize,
        modifier: M,
        filter: F,
    ) {
        for x in 0..WIDTH {
            let item = self.get_at_mut(x, y);
            if filter(item) {
                modifier(item);
            }
        }
    }

    fn mutable_iterator<'a>(&'a mut self) -> impl Iterator<Item = &'a mut GridItem> + 'a
    where
        GridItem: 'a;
}
