# Validación independiente del núcleo (spec 001)

> Fecha: 2026-11-11 · Alcance: `crates/almarioks-core`, solo su API pública · Pruebas: `crates/almarioks-core/tests/spec_*.rs`
> (ayudantes en `tests/spec_support/mod.rs`). Las pruebas se escribieron desde `spec.md` (definiciones, RF, ejemplos
> trabajados y casos límite) sin leer los cuerpos de las funciones ni las pruebas existentes. El valor esperado de
> cada prueba sale de la spec; donde la spec no decide, la prueba no afirma nada (ver "Spec ambigua").
>
> Comando: `cargo nextest run -p almarioks-core --locked -E 'test(/^spec_/)'`. El defecto queda como `#[ignore]`;
> se ve con `--run-ignored all`.

## Cómo se probó lo difícil
- **Disco de prueba propio** (`FaultDisk`, implementa el `trait Disk` público): por operación pasa, falla, se cuelga
  hasta que la prueba lo suelta o corrompe (escribe la mitad y dice que funcionó). Distingue el archivo de
  recuperación por su nombre (`.recuperacion-`, RF-33) y puede fallar en la k-ésima operación, sola o "muriendo"
  desde ahí (RF-27).
- **Reloj propio** (`ManualClock`): no avanza solo y guarda las alarmas hasta que la prueba las suena. Ningún `sleep`:
  canales con tiempo máximo y la señal `SessionEvents::locked`.
- **Otra app KeePass**: el crate `keepass` reescribe el archivo (RF-24), fabrica campos protegidos (título, usuario,
  URL, notas, campo propio), la papelera dentro de otro grupo (RF-57) y KDF débiles (RF-17), y lee historial y objetos
  borrados, que no tienen API pública.
- **Cabeceras hostiles**: se parte de una bóveda creada por el núcleo, se reescribe el TLV de la cabecera KDBX 4 o el
  `VariantDictionary` del KDF y se recalcula el SHA-256 de la cabecera, para que solo falle el campo bajo prueba. Se
  abre con la llave correcta y se exige respuesta en menos de 1 s (no se derivó).

## Resultado por requisito
| RF | Prueba `spec_` | Resultado |
|---|---|---|
| RF-1 | `spec_rf_1_rf_2_a_new_vault_reopens_with_its_key_only_and_uses_argon2id` | pasa |
| RF-2 | ídem + `spec_rf_2_calibration_picks_the_iterations_closest_to_one_second` | pasa |
| RF-3, RF-4, RF-6 | `spec_rf_3_rf_4_rf_6_creation_refuses_without_touching_anything` | pasa |
| RF-5 | `spec_rf_5_a_new_key_file_is_keyx_v2_with_32_bytes_and_opens_the_vault` | pasa |
| RF-11 (ej. 7) | `spec_rf_11_example_7_three_wrong_keys_give_the_same_code_and_text` (núcleo y sesión) | pasa |
| RF-12 | `spec_rf_12_a_keyx_whose_hash_does_not_match_is_archivo_llave_danado` | pasa |
| RF-13 | `spec_rf_13_empty_plain_text_and_truncated_files_are_archivo_danado` · `…_header_that_does_not_verify…` · `…_kdb_1_kdbx_2_and_kdbx_5…` | pasa |
| RF-14 | `spec_rf_14_a_kdbx_3_vault_opens_read_only_and_every_change_is_solo_lectura` (fixture KeePass 2) | pasa |
| RF-15 | `spec_rf_15_converting_writes_a_new_kdbx_4_…_leaves_the_original_identical` | pasa |
| RF-16 | `spec_rf_16_hostile_kdf_parameters_are_rejected_before_deriving` (64 GiB, 1 000 000 iteraciones, 17 hilos, AES 100 000 001) · `spec_rf_16_the_limits_themselves_are_allowed` | pasa |
| RF-17 | `spec_rf_17_a_weak_kdf_opens_with_a_warning` (AES-KDF, Argon2id 32 MiB, Argon2d) | pasa |
| RF-18 | `spec_rf_88_rf_18_…` (estado en < 200 ms con el desbloqueo detenido en el disco) | pasa (parcial: no hay derivación lenta inyectable) |
| RF-21 | `spec_rf_21_a_temporary_file_that_does_not_read_back_as_written_is_never_put_in_place` · `…_in_the_same_folder_as_the_vault` | pasa |
| RF-22 (ej. 6) | `spec_rf_22_example_6_the_bak_holds_the_previous_version_and_opens_with_the_same_key` | pasa |
| RF-23 | `spec_rf_23_a_failure_at_any_step_leaves_the_original_byte_for_byte` · `…_keeps_the_change_in_memory_editable_and_retry_saves_it` · `…_save_as_never_replaces_an_existing_file…` | pasa |
| RF-24 | `spec_rf_24_a_file_rewritten_by_another_app_is_not_overwritten_and_save_as_keeps_the_change` · `…_reload_…` · `…_only_changes_the_date…` | pasa |
| RF-26 | `spec_rf_26_history_is_oldest_first_and_keeps_within_the_vault_limit` · ej. 8 | pasa |
| RF-27 | `spec_rf_27_if_the_process_dies_at_any_step_the_vault_file_still_opens_with_its_key` | pasa |
| RF-28 | `spec_rf_28_rf_35_after_lock_every_command_answers_bloqueada_and_nothing_is_written` | pasa |
| RF-30 | `spec_rf_30_rf_33_lock_with_a_hung_save_…_within_1s…` · `spec_rf_30_a_lock_asked_during_an_unlock_leaves_the_vault_locked` · `spec_rf_33_a_hung_recovery_write…` | pasa |
| RF-33 | `spec_rf_30_rf_33_…` · `spec_rf_33_rf_34_failed_save_goes_to_a_new_recovery_file_each_time…` · `spec_rf_33_a_hung_recovery_write_does_not_hold_the_lock_beyond_its_limit` · `spec_rf_33_the_recovery_holds_only_encrypted_bytes…` | pasa |
| RF-34 | `spec_rf_33_rf_34_…` (una sola vez) · `spec_rf_34_when_the_recovery_cannot_be_written_unlock_says_the_changes_were_lost` | pasa |
| RF-35 | `spec_rf_35_closed_session_answers_bloqueada_to_every_vault_command` · `spec_rf_28_rf_35_…` | pasa |
| RF-36 | `spec_rf_36_close_does_the_same_as_lock_and_goes_back_to_welcome` | pasa |
| RF-39, RF-40 (ej. 9) | `spec_rf_39_rf_40_example_9_titles` · `spec_rf_40_each_limit_names_its_field_and_limit_and_counts_characters` | pasa |
| RF-41 | `spec_rf_41_the_password_is_stored_exactly_as_written` | pasa |
| RF-42 | `spec_rf_42_lists_are_in_title_order_folded_then_by_uuid` | pasa |
| RF-43 | `spec_rf_43_rf_76_tree_lists_detail_and_debug_carry_no_protected_value` · `spec_rf_43_detail_names_custom_fields…` | pasa |
| RF-44 | `spec_rf_44_showing_a_field_gives_only_that_value_of_that_entry` | pasa |
| RF-45 (ej. 8) | `spec_rf_45_example_8_editing_only_the_title_keeps_the_password_and_adds_one_version` · `…_replaced_also_by_empty…` | pasa |
| RF-46 | `spec_rf_46_moving_an_entry_leaves_it_only_in_the_target_and_adds_no_version` | pasa |
| RF-47, RF-48 | `spec_rf_47_rf_48_delete_goes_to_a_new_bin_then_for_good_into_deleted_objects` | pasa |
| RF-52, RF-53 | `spec_rf_52_rf_53_group_names_are_trimmed_and_bounded` | pasa |
| RF-54, RF-55 (ej. 3) | `spec_rf_55_rf_54_example_3_moving_groups` | pasa |
| RF-56 (ej. 4) | `spec_rf_56_rf_58_example_4_deleting_a_group_counts_every_level_then_bin_then_for_good` | pasa |
| RF-57 | `spec_rf_57_root_and_bin_are_protected_and_the_root_may_be_renamed` · `spec_rf_57_a_group_that_holds_the_bin_cannot_be_deleted` | pasa |
| RF-58 | `spec_rf_56_rf_58_…` · `spec_rf_58_emptying_the_bin_removes_everything_in_it_and_keeps_the_bin` | pasa |
| RF-59, RF-60, RF-63 (ej. 2) | 6 pruebas `spec_rf_59_…`/`spec_rf_60_…`/`spec_rf_63_…` (incluye `anos` → "Años", `ana` → "Banana", URL, etiquetas, ruta del grupo, empate por UUID) | pasa |
| RF-61 | `spec_rf_61_search_never_looks_inside_a_protected_field_and_hits_carry_no_secret` | pasa |
| RF-62 | `spec_rf_62_entries_in_the_bin_are_excluded` | pasa |
| RF-65 a RF-67 (ej. 5) | `spec_rf_66_rf_67_example_5_…` · `spec_rf_65_defaults…` · `spec_rf_65_rf_66_every_subset_of_types_at_both_length_limits` | pasa |
| RF-68 | `spec_rf_68_no_type_or_length_outside_8_to_128_is_reglas_imposibles` | pasa |
| RF-69 | `spec_rf_69_every_allowed_character_is_equally_likely_within_20_percent` (100 000 muestras, semilla fija) · `…_a_source_that_can_never_satisfy…` | pasa |
| RF-7, "Fortaleza" | `spec_fortaleza_anchors_and_rf_7_weak_below_3` (solo el umbral del núcleo; el aviso es de la ventana) | pasa |
| RF-19 | `spec_rf_19_remembers_the_last_5_vaults_most_recent_first_and_survives_a_round_trip` | pasa |
| RF-29, RF-73 (valores) | `spec_rf_29_rf_73_only_the_allowed_values_can_be_set` | pasa (solo los valores; el temporizador no está en el núcleo) |
| RF-76 | `spec_rf_76_debug_of_keys_inputs_and_session_never_prints_a_secret` · `spec_rf_43_rf_76_…` | pasa |
| RF-77 (recuperación) | `spec_rf_33_the_recovery_holds_only_encrypted_bytes…` | pasa (solo el archivo de recuperación) |
| RF-82 | `spec_rf_82_damaged_settings_fall_back_to_defaults_field_by_field` | pasa |
| RF-87 | `spec_rf_87_key_files_in_every_keepass_format_give_the_same_key` (32 bytes, 64 hex, `.keyx` v2 y "otro archivo" = su SHA-256) | pasa |
| RF-88 | `spec_rf_88_rf_18_a_second_unlock_while_the_first_runs_is_ocupada` | pasa |
| RF-89 | `spec_rf_89_a_change_made_while_a_save_hangs_waits_its_turn_and_both_end_in_the_file` | pasa |
| RF-90 | `spec_rf_90_a_value_read_when_the_lock_arrives_is_answered_bloqueada` | pasa |
| RF-91 | `spec_rf_91_tags_with_separators_or_blank_are_rejected_and_others_are_trimmed` | pasa |
| RF-93, RF-80 | `spec_rf_93_unknown_ids_answer_no_encontrado_before_any_domain_rule` | pasa |
| RF-94 | `spec_rf_94_a_header_field_of_the_wrong_size_is_archivo_danado_and_the_process_survives` | pasa |
| RF-94 | `spec_rf_94_a_master_seed_or_iv_of_the_wrong_size_is_archivo_danado` | **DEFECTO** (ignorada) |
| RF-95 | `spec_rf_95_missing_vault_or_key_file_says_which` | pasa |
| Definiciones (control) | `spec_definiciones_control_characters_are_rejected_but_tab_and_line_feed_are_not` | pasa |
| RF-10 | — | no se pudo probar aquí: las bóvedas de KeePass 2/KeePassXC están en `tests/fixtures` pero sin la llave documentada para todas; las cubren las pruebas `roundtrip_*` del implementador (no leídas) |
| RF-25 | — | no probado (ida y vuelta completa: es de las pruebas `roundtrip_*`) |
| RF-29 (tiempo), RF-31, RF-32, RF-37 | — | fuera del núcleo: el temporizador, el portapapeles y la vista son de la app; del núcleo solo se comprobó que avisa `SessionEvents::locked` |
| RF-49, RF-71 a RF-75, RF-78, RF-79, RF-81, RF-83, RF-84 | — | fuera del núcleo o sin API pública en `almarioks-core` (URL, portapapeles, ventana, Tauri) |
| D12 (orden: soltar llaves antes de escribir) | — | no observable desde la API pública: no hay forma de saber si la llave sigue en memoria mientras se escribe la recuperación |
| RF-8, RF-9, RF-20, RF-64, RF-70 | — | de la ventana (texto y estados vacíos) |

## Defectos
### D-1 · RF-94 · Bajo — un campo de cabecera de tamaño equivocado responde `LlaveIncorrecta`
- **La spec dice:** "SI la cabecera trae un campo de tamaño distinto al que su tipo exige, ENTONCES EL SISTEMA
  responderá `ArchivoDanado` y la app seguirá funcionando." (RF-94).
- **Qué hace el núcleo:** con el `MasterSeed` (campo 4) de 31 bytes en vez de 32, o el IV de ChaCha20 (campo 7) de
  11 bytes en vez de 12, y la suma SHA-256 de la cabecera recalculada, `kdbx::open` con la llave correcta responde
  `LlaveIncorrecta`. No se cae ni deriva un KDF hostil; el daño es el mensaje: le dice a la persona que su llave está
  mal cuando el archivo está dañado, y la empuja a reintentar contraseñas. Los campos de tamaño fijo que la librería
  lee con `byteorder` (cifrado, compresión, ítems del KDF) sí dan `ArchivoDanado`.
- **Entrada mínima:** una bóveda creada con `kdbx::create_vault`, el campo 4 truncado a 31 bytes, SHA-256 de la
  cabecera recalculado → `kdbx::open(bytes, llave_correcta)` = `Err(LlaveIncorrecta)`.
- **Matiz:** la fila de casos límite "Cabecera cambiada por alguien que recalcula su suma → `LlaveIncorrecta`" cubre
  cambiar *valores*; un *tamaño* inválido lo nombra RF-94 de forma específica y KeePass 2 lo rechaza como error de
  formato. Si el dueño decide que basta con no caerse, se cierra cambiando la prueba, no el código.

Ningún otro defecto: las 81 pruebas restantes pasan.

## API poco clara (no son defectos)
- **Edición sin cambios y guardado:** `update_entry` devuelve `bool` y quien llama a `Session::mutate` decide entre
  `Applied::Changed` y `Applied::Unchanged`. Si la capa de comandos envuelve siempre en `Changed`, una edición sin
  cambios reescribe el archivo y pisa el `.bak` con el mismo contenido (RF-22, RF-45). El núcleo no lo impide; conviene
  una prueba en la capa de comandos.
- **Historial y objetos borrados** no tienen lectura pública (RF-85 es recortable); para RF-26, RF-46, RF-48 y RF-58
  hubo que releer `to_bytes` con `keepass`.
- **Bloqueo con la recuperación colgada:** si la escritura no termina en el límite, `lock()` no dice `Saved` (bien),
  pero la doc avisa que el archivo "aún puede aparecer". La spec (RF-34) solo conoce "quedó en un archivo" o "se
  perdió"; falta decidir qué se le dice a la persona en ese caso.
- **Nombre del archivo de recuperación:** UTC u hora local está pendiente en la doc (`Timestamp`). Las pruebas solo
  exigen el patrón `AAAAMMDD-HHMMSS`, y que un segundo archivo no pise al primero (el sufijo `-2` es de la
  implementación, no de la spec).
- **`Status::Deriving`** no se ve mientras el desbloqueo lee el archivo (solo se exigió "no abierta" y respuesta en
  < 200 ms); con un disco lento la ventana podría no mostrar el progreso de RF-18.

## Spec ambigua (las pruebas no afirman nada aquí)
- Contraseña maestra vacía: el núcleo la cuenta como "sin contraseña" (`LlaveVacia`); la spec (RF-3) no lo dice.
- Precedencia entre `GrupoProtegido` y `MovimientoInvalido` (mover la raíz dentro de un descendiente).
- Si la papelera existe pero la bóveda la tiene desactivada: RF-47 dice "creándola y activándola si no existe".
- Un archivo de bóveda que desaparece antes de guardar: ¿`CambioExterno` (RF-24) o `GuardadoFallido`?
- U+0085 (NEL) es a la vez `White_Space` y carácter de control: ¿se recorta o se rechaza?

## Lectura de código fuera de las firmas
- Firmas públicas y `///` de los módulos indicados (con `grep`); algunas búsquedas mostraron de paso líneas sueltas de
  cuerpos de `session.rs` y `storage/*.rs` (ramas `Ok`/`Err`), sin leer funciones enteras.
- Para fabricar bóvedas con `keepass` 0.17: el laboratorio `docs/lab/boveda-0.1/src/bin/p4_keepass.rs` (llaves y
  configuración) y las firmas del propio crate `keepass` en el registro de cargo. La contraseña del fixture KDBX 3.1
  salió de `docs/lab/kdbx-roundtrip/correr.ps1`.
- No se leyeron `tests/rf_*.rs` ni `src/**/tests*.rs`.
