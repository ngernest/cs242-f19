#[allow(unused_imports)]
use crate::backend::{login, order, UserId};

// Each state in the DFA corresponds to a different struct
pub struct Cart;
pub struct Empty;
pub struct NonEmpty {
  items: Vec<f64>,
}
pub struct Checkout {
  items: Vec<f64>,
}

impl Cart {
  pub fn login(username: String, password: String) -> Option<Empty> {
    if let Ok(_) = login(username, password) {
      Some(Empty)
    } else {
      None
    }
  }
}

impl Empty {
  pub fn additem(self, cost: f64) -> NonEmpty {
    NonEmpty { items: vec![cost] }
  }
}

impl NonEmpty {
  pub fn additem(mut self, cost: f64) -> NonEmpty {
    self.items.push(cost);
    self
  }

  pub fn clearitems(mut self) -> NonEmpty {
    self.items.clear();
    self
  }

  /// Computes the total cost of all the items in the cart
  pub fn checkout(self) -> Checkout {
    Checkout { items: self.items }
  }
}

impl Checkout {
  pub fn cancel(self) -> NonEmpty {
    NonEmpty { items: self.items }
  }

  pub fn order(self) -> Result<Empty, (Checkout, String)> {
    let total: f64 = self.items.iter().sum();
    if total >= 0.0 {
      Ok(Empty)
    } else {
      Err((self, "total cost is negative".to_string()))
    }
  }
}
