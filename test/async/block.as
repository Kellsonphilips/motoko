let a = async {
   let (a,b) = ("a1","b1"); 
   print a;
   print b;
};

let b = async {
   let (a,b) = await (async ("a2","b2"));
   print a;
   print b;
};

let c = async {
   func f(a:Text,b:Text):(){ print a; print b;};
   let (a,b) = await (async ("a3","b3"));
   let _ = f(a,b);
};

let d = async {
   var f = 1;
   printInt (f);
   let (a,b) = await (async ("a4","b4"));
   f += 2;
   printInt (f);
};


let e = async {
   var f = await (async 5);
   printInt (f);
   let (a,b) = await (async ("a5","b5"));
   f += 1;
   printInt (f);
};

let f = async {
   class point(x:Int,y:Int) {
     get_x():Int = x;
     get_y():Int = y;
     move(dx:Int,dy:Int): point {
       point (get_x()+dx, get_y()+dy);
     };        
   };
   let p = point(666,666);
   printInt (p.get_x());
   printInt (p.get_y());
   let (a,b) = await (async ("a6","b6"));
   assert( p is point);
   let q = p.move(1,-1);   
   printInt (q.get_x());
   printInt (q.get_y());
};
