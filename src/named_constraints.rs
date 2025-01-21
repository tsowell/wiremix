macro_rules! with_named_constraints {
    ($constraints:expr, $closure:expr) => {{
        let mut vec = Vec::new();
        let mut names = Vec::new();
        let mut index = 0;
        for constraint in $constraints {
            match constraint {
                (constraint, Some::<&mut Rect>(var)) => {
                    names.push((var, index));
                    vec.push(constraint)
                }
                (constraint, None) => vec.push(constraint),
            }
            index += 1;
        }
        let layout = $closure(vec);
        for (var, index) in names {
            *var = layout[index];
        }
        layout
    }};
}

pub(crate) use with_named_constraints;
