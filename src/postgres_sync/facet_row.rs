use facet_core::Facet;
use facet_core::Shape;
use facet_core::StructKind;
use facet_core::Type;
use facet_core::UserType;
use facet_reflect::AllocError;
use facet_reflect::Partial;
use facet_reflect::ReflectError;
use facet_reflect::ShapeMismatchError;
use tokio_postgres::Row;

#[derive(Debug)]
pub enum Error {
    MissingColumn {
        column: String,
    },
    TypeMismatch {
        column: String,
        expected: &'static Shape,
        source: tokio_postgres::Error,
    },
    Reflect(ReflectError),
    Alloc(AllocError),
    ShapeMismatch(ShapeMismatchError),
    NotAStruct {
        shape: &'static Shape,
    },
    UnsupportedType {
        field: String,
        shape: &'static Shape,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingColumn { column } => write!(f, "missing column: {column}"),
            Error::TypeMismatch {
                column, expected, ..
            } => write!(
                f,
                "type mismatch for column '{column}': expected {expected}"
            ),
            Error::Reflect(error) => write!(f, "reflection error: {error}"),
            Error::Alloc(error) => write!(f, "allocation error: {error}"),
            Error::ShapeMismatch(error) => write!(f, "shape mismatch: {error}"),
            Error::NotAStruct { shape } => {
                write!(f, "cannot deserialize row into non-struct type: {shape}")
            }
            Error::UnsupportedType { field, shape } => {
                write!(f, "unsupported type for field '{field}': {shape}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::TypeMismatch { source, .. } => Some(source),
            Error::Reflect(error) => Some(error),
            Error::Alloc(error) => Some(error),
            Error::ShapeMismatch(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ReflectError> for Error {
    fn from(error: ReflectError) -> Self {
        Self::Reflect(error)
    }
}

impl From<AllocError> for Error {
    fn from(error: AllocError) -> Self {
        Self::Alloc(error)
    }
}

impl From<ShapeMismatchError> for Error {
    fn from(error: ShapeMismatchError) -> Self {
        Self::ShapeMismatch(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Deserialize a Postgres row into a Facet-backed struct.
///
/// # Errors
///
/// Returns an error if the row cannot be mapped onto the destination shape.
pub fn from_row<T: Facet<'static>>(row: &Row) -> Result<T> {
    let partial = Partial::alloc_owned::<T>()?;
    let partial = deserialize_row_into(row, partial, T::SHAPE)?;
    let heap_value = partial.build()?;
    Ok(heap_value.materialize()?)
}

fn deserialize_row_into(
    row: &Row,
    partial: Partial<'static, false>,
    shape: &'static Shape,
) -> Result<Partial<'static, false>> {
    let struct_def = match &shape.ty {
        Type::User(UserType::Struct(struct_def)) if struct_def.kind == StructKind::Struct => {
            struct_def
        }
        _ => return Err(Error::NotAStruct { shape }),
    };

    let mut partial = partial;
    for (index, field) in struct_def.fields.iter().enumerate() {
        let column_name = field.rename.unwrap_or(field.name);
        let Some(column_index) = row
            .columns()
            .iter()
            .position(|column| column.name() == column_name)
        else {
            if field.shape().decl_id == Option::<()>::SHAPE.decl_id {
                partial = partial.set_nth_field_to_default(index)?;
                continue;
            }

            return Err(Error::MissingColumn {
                column: column_name.to_string(),
            });
        };

        partial = partial.begin_field(field.name)?;
        partial = deserialize_column(row, column_index, column_name, partial, field.shape())?;
        partial = partial.end()?;
    }

    Ok(partial)
}

fn deserialize_column(
    row: &Row,
    column_index: usize,
    column_name: &str,
    mut partial: Partial<'static, false>,
    shape: &'static Shape,
) -> Result<Partial<'static, false>> {
    if shape.decl_id == Option::<()>::SHAPE.decl_id {
        return deserialize_option_column(row, column_index, column_name, partial, shape);
    }

    if shape == i8::SHAPE {
        partial = partial.set(get_column::<i8>(row, column_index, column_name, shape)?)?;
    } else if shape == i16::SHAPE {
        partial = partial.set(get_column::<i16>(row, column_index, column_name, shape)?)?;
    } else if shape == i32::SHAPE {
        partial = partial.set(get_column::<i32>(row, column_index, column_name, shape)?)?;
    } else if shape == i64::SHAPE {
        partial = partial.set(get_column::<i64>(row, column_index, column_name, shape)?)?;
    } else if shape == f32::SHAPE {
        partial = partial.set(get_column::<f32>(row, column_index, column_name, shape)?)?;
    } else if shape == f64::SHAPE {
        partial = partial.set(get_column::<f64>(row, column_index, column_name, shape)?)?;
    } else if shape == bool::SHAPE {
        partial = partial.set(get_column::<bool>(row, column_index, column_name, shape)?)?;
    } else if shape == String::SHAPE {
        partial = partial.set(get_column::<String>(row, column_index, column_name, shape)?)?;
    } else if shape.vtable.has_parse() {
        let value: String = get_column(row, column_index, column_name, shape)?;
        partial = partial.parse_from_str(&value)?;
    } else {
        return Err(Error::UnsupportedType {
            field: column_name.to_string(),
            shape,
        });
    }

    Ok(partial)
}

fn deserialize_option_column(
    row: &Row,
    column_index: usize,
    column_name: &str,
    mut partial: Partial<'static, false>,
    shape: &'static Shape,
) -> Result<Partial<'static, false>> {
    let inner_shape = shape.inner.expect("Option must have inner shape");

    macro_rules! try_option {
        ($type:ty) => {{
            let value: Option<$type> = get_column(row, column_index, column_name, shape)?;
            match value {
                Some(value) => {
                    partial = partial.begin_some()?;
                    partial = partial.set(value)?;
                    partial = partial.end()?;
                }
                None => {
                    partial = partial.set_default()?;
                }
            }
            return Ok(partial);
        }};
    }

    if inner_shape == i8::SHAPE {
        try_option!(i8);
    } else if inner_shape == i16::SHAPE {
        try_option!(i16);
    } else if inner_shape == i32::SHAPE {
        try_option!(i32);
    } else if inner_shape == i64::SHAPE {
        try_option!(i64);
    } else if inner_shape == f32::SHAPE {
        try_option!(f32);
    } else if inner_shape == f64::SHAPE {
        try_option!(f64);
    } else if inner_shape == bool::SHAPE {
        try_option!(bool);
    } else if inner_shape == String::SHAPE {
        try_option!(String);
    }

    if inner_shape.vtable.has_parse() {
        let value: Option<String> = get_column(row, column_index, column_name, shape)?;
        match value {
            Some(value) => {
                partial = partial.begin_some()?;
                partial = partial.parse_from_str(&value)?;
                partial = partial.end()?;
            }
            None => {
                partial = partial.set_default()?;
            }
        }
        return Ok(partial);
    }

    Err(Error::UnsupportedType {
        field: column_name.to_string(),
        shape: inner_shape,
    })
}

fn get_column<'row, T>(row: &'row Row, index: usize, name: &str, shape: &'static Shape) -> Result<T>
where
    T: postgres_types::FromSql<'row>,
{
    row.try_get::<_, T>(index)
        .map_err(|source| Error::TypeMismatch {
            column: name.to_string(),
            expected: shape,
            source,
        })
}
