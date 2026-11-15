"""Reverificacion de D-1 y D-2: que dejo WebView2 en disco.

Uso: py -3 -I rv-inspeccionar.py nombre=carpeta [nombre=carpeta ...]
Los secretos a buscar llegan por la variable de entorno RV_SECRETS (JSON en base64), nunca por un archivo:
  {"plain": {nombre: valor}, "hashed": {nombre: valor}}
- "plain": se busca el valor en claro, en UTF-8 y en UTF-16LE, dentro de cada archivo.
- "hashed": ademas se buscan los primeros 8 bytes de su SHA-1 (decimal, hexadecimal y crudos) y el SHA-1 completo.
RV_SINCE (segundos Unix, opcional): solo archivos modificados desde entonces. RV_EXCLUDE: prefijos de ruta separados por "|".
Cada base SQLite se abre en solo lectura y se cuentan las filas de sus tablas "autofill*".
Solo imprime nombres de aguja y rutas relativas a la carpeta dada: ningun valor ni ruta del PC.
"""
import base64
import hashlib
import json
import os
import pathlib
import sqlite3
import struct
import sys

sys.stdout.reconfigure(encoding="utf-8")


def build_needles(cfg):
    needles = []
    for name, value in cfg.get("plain", {}).items():
        needles.append((f"{name}, en claro (UTF-8)", value.encode("utf-8")))
        needles.append((f"{name}, en claro (UTF-16LE)", value.encode("utf-16-le")))
    hashed = {}
    for name, value in cfg.get("hashed", {}).items():
        raw = value.encode("utf-8")
        needles.append((f"{name}, en claro (UTF-8)", raw))
        needles.append((f"{name}, en claro (UTF-16LE)", value.encode("utf-16-le")))
        digest = hashlib.sha1(raw).digest()
        unsigned = str(struct.unpack(">Q", digest[:8])[0])
        signed = str(struct.unpack(">q", digest[:8])[0])
        hashed[unsigned] = name
        hashed[signed] = name
        needles.append((f"{name}, SHA-1 8 bytes en decimal (UTF-8)", unsigned.encode()))
        needles.append((f"{name}, SHA-1 8 bytes en decimal (UTF-16LE)", unsigned.encode("utf-16-le")))
        if signed != unsigned:
            needles.append((f"{name}, SHA-1 8 bytes en decimal con signo", signed.encode()))
        needles.append((f"{name}, SHA-1 8 bytes crudos", digest[:8]))
        needles.append((f"{name}, SHA-1 8 bytes crudos al reves", digest[:8][::-1]))
        needles.append((f"{name}, SHA-1 8 bytes en hex", digest[:8].hex().encode()))
        needles.append((f"{name}, SHA-1 8 bytes en HEX", digest[:8].hex().upper().encode()))
        needles.append((f"{name}, SHA-1 completo en hex", digest.hex().encode()))
        needles.append((f"{name}, SHA-1 completo crudo", digest))
    return needles, hashed


def inspect_sqlite(root_name, rel, path, hashed):
    try:
        con = sqlite3.connect(pathlib.Path(path).as_uri() + "?mode=ro&immutable=1", uri=True)
    except sqlite3.Error as error:
        print(f"SQLITE|{root_name}|{rel}|no se pudo abrir: {error}")
        return
    try:
        cur = con.cursor()
        tables = [row[0] for row in cur.execute("select name from sqlite_master where type='table'")]
        counts = {}
        for table in tables:
            if "autofill" in table.lower():
                counts[table] = cur.execute(f'select count(*) from "{table}"').fetchone()[0]
        total = sum(counts.values())
        detail = ", ".join(f"{t}={n}" for t, n in sorted(counts.items()) if n) or "todas vacias"
        print(f"SQLITE|{root_name}|{rel}|tablas={len(tables)}|tablas autofill={len(counts)}|filas autofill={total}|{detail}")
        if counts.get("autofill_edge_field_values"):
            masked = cur.execute("select count(*) from autofill_edge_field_values where is_masked=1").fetchone()[0]
            clear = cur.execute("select count(*) from autofill_edge_field_values where is_masked=0").fetchone()[0]
            print(f"SQLITE|{root_name}|{rel}|autofill_edge_field_values: en claro={clear}, enmascaradas={masked}")
            for (value,) in cur.execute("select value from autofill_edge_field_values where is_masked=1"):
                if str(value) in hashed:
                    print(f"COINCIDE_HASH|{root_name}|{rel}|una fila enmascarada es el SHA-1 de 8 bytes de: {hashed[str(value)]}")
        if counts.get("autofill_edge_field_client_info"):
            labels = [row[0] for row in cur.execute("select distinct label from autofill_edge_field_client_info")]
            print(f"SQLITE|{root_name}|{rel}|campos recordados: {' / '.join(str(label) for label in labels)}")
    except sqlite3.Error as error:
        print(f"SQLITE|{root_name}|{rel}|error al leer: {error}")
    finally:
        con.close()


def scan(root_name, root, needles, hashed, since, excluded):
    files = 0
    size = 0
    unreadable = 0
    hits = 0
    sqlite_files = 0
    top = {}
    paths = []
    if os.path.isfile(root):
        paths.append(root)
    else:
        for folder, _dirs, names in os.walk(root):
            for name in names:
                paths.append(os.path.join(folder, name))
    for path in paths:
        low = path.lower()
        if any(low.startswith(prefix) for prefix in excluded):
            continue
        try:
            stat = os.stat(path)
            if since and stat.st_mtime < since:
                continue
            if stat.st_size > 400 * 1024 * 1024:
                unreadable += 1
                continue
            with open(path, "rb") as handle:
                data = handle.read()
        except OSError:
            unreadable += 1
            continue
        rel = os.path.relpath(path, root) if os.path.isdir(root) else os.path.basename(path)
        files += 1
        size += len(data)
        first = rel.split(os.sep)[0] if os.sep in rel else "(raiz)"
        second = os.sep.join(rel.split(os.sep)[:2]) if rel.count(os.sep) >= 2 else first
        top[second] = top.get(second, 0) + 1
        for name, needle in needles:
            if needle in data:
                hits += 1
                print(f"APARICION|{root_name}|{rel}|{name}")
        if data[:16] == b"SQLite format 3\x00":
            sqlite_files += 1
            inspect_sqlite(root_name, rel, path, hashed)
    if os.environ.get("RV_TREE") == "1":
        for folder, count in sorted(top.items()):
            print(f"ARBOL|{root_name}|{folder}|{count} archivos")
    print(f"RESUMEN|{root_name}|archivos={files}|MB={size / 1048576:.1f}|sin leer={unreadable}|sqlite={sqlite_files}|apariciones={hits}")


def main():
    cfg = json.loads(base64.b64decode(os.environ["RV_SECRETS"]).decode("utf-8"))
    needles, hashed = build_needles(cfg)
    since = float(os.environ.get("RV_SINCE") or 0)
    excluded = [p.lower() for p in (os.environ.get("RV_EXCLUDE") or "").split("|") if p]
    for arg in sys.argv[1:]:
        name, _, root = arg.partition("=")
        if not os.path.exists(root):
            print(f"RESUMEN|{name}|no existe")
            continue
        scan(name, root, needles, hashed, since, excluded)


main()
