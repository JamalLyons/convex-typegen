use convex_typegen_runtime::ConvexValue;

#[derive(ConvexValue)]
struct Tuple(i64);

#[derive(ConvexValue)]
struct Generic<T> {
    field: T,
}

#[derive(ConvexValue)]
enum UnitBranch {
    Null,
}

#[derive(ConvexValue)]
#[convex(literal = 1_u32)]
struct UnsupportedLiteral;

#[derive(ConvexValue)]
#[convex(discriminator = "kind")]
enum MissingTag {
    First(String),
}

#[derive(ConvexValue)]
struct DuplicateField {
    name: String,
    #[convex(rename = "name")]
    other: String,
}

fn main() {}
