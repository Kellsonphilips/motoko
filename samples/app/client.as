actor class Client() = this {
  // TODO: these should be constructor params once we can compile them
  private var name : Text = "";
  private var server : ?Server  = null;

  go(n : Text, s : Server) {
    name := n;
    server := ?s;
    ignore(async {
      let sub = await s.subscribe(this.send);
      sub.post("hello from " # name);
      sub.post("goodbye from " # name);
      sub.cancel();
    })
  };

  send(msg : Text) {
    print(name # " received " # msg # "\n");
  };
};

