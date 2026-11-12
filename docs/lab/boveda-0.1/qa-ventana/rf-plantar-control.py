"""Control positivo del inspector: fabrica una base SQLite con la forma de "Web Data" de WebView2.

Uso: py -3 -I rf-plantar-control.py carpeta
Lee RV_SECRETS (el mismo JSON en base64 que rv-inspeccionar.py) y escribe carpeta/Web Data con la tabla
autofill_edge_field_values: una fila en claro con el valor "titulo" de "plain" y una fila enmascarada con los
primeros 8 bytes del SHA-1 del valor "maestra" de "hashed", en decimal, que es como WebView2 guardaba la repeticion
de la contrasena maestra (defecto D-1).
"""
import base64
import hashlib
import json
import os
import sqlite3
import struct
import sys

cfg = json.loads(base64.b64decode(os.environ["RV_SECRETS"]).decode("utf-8"))
plain = cfg["plain"]["titulo"]
master = cfg["hashed"]["maestra"]
digest = hashlib.sha1(master.encode("utf-8")).digest()
masked = str(struct.unpack(">Q", digest[:8])[0])
con = sqlite3.connect(os.path.join(sys.argv[1], "Web Data"))
con.execute("create table autofill_edge_field_values (field_id integer, value text, is_masked integer)")
con.execute("create table autofill_edge_field_client_info (field_id integer, label text)")
con.execute("insert into autofill_edge_field_values values (1, ?, 0)", (plain,))
con.execute("insert into autofill_edge_field_values values (2, ?, 1)", (masked,))
con.execute("insert into autofill_edge_field_client_info values (1, 'Título')")
con.commit()
con.close()
print("PLANTADO|Web Data de control: 2 filas (1 en claro, 1 enmascarada)")
