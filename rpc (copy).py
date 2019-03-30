from bitcoinrpc.authproxy import AuthServiceProxy, JSONRPCException

rpc_connection = AuthServiceProxy("http://%s:%s@127.0.0.1:8332"%('username', 'password'), timeout = 500)

for i in range(0,567032):
 bbh = rpc_connection.getblockhash(i)
 bh = rpc_connection.getblockheader(bbh)
 print bh["height"],bh["time"],bh["nTx"],bh["difficulty"],bh["nonce"]
