use serde::{ser::SerializeTuple, Serialize, Serializer};
use std::ops::Deref;

pub struct Node<'a, T>(pub &'a trees::Node<T>);

impl<'a, T> Deref for Node<'a, T> {
    type Target = trees::Node<T>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T: Serialize> Serialize for Node<'a, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let arity = 1 + self.degree();
        if arity == 1 {
            self.data().serialize(serializer)
        } else {
            let mut tup = serializer.serialize_tuple(arity)?;
            tup.serialize_element(self.data())?;
            for child in self.iter() {
                tup.serialize_element(&Node(child))?;
            }
            tup.end()
        }
    }
}

// fn main() {
//     use trees::tr;
//
//     let tree = tr(0) /( tr(1)/tr(2)/tr(3) ) /( tr(4)/tr(5)/tr(6) );
//     println!( "{}", serde_json::to_string( &Node( tree.root() )).unwrap() );
// }
