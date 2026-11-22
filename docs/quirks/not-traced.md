# Quirks not traced to a C line

[Back to the register index](../QUIRKS.md).

None is left. The sweep carried candidates that cited no line or that the
sweeper could not verify; the merge traced every one by opening the C:
the dying barrel (`info.c:944-948`, `p_inter.c:675`, `p_enemy.c:1588`), the
`A_Explode` freed-memory read (`p_enemy.c:1600`, `p_map.c:1227`), "only the
last motion event of a tic counts" (`g_game.c:576-580`), "fire can stick"
(`i_video.c:218-247`), the stale `attacker` pointer (the absent clear in
`P_SpawnPlayer` and `G_PlayerFinishLevel`, the writer `p_inter.c:874` and the
reader `p_user.c:200-205`), the sector sound-origin overflow
(`p_setup.c:556-557`), the diagonal-wall "sliver" reading quirks
(`p_maputl.c:91-99`, `139-152`), the inverting door (`p_doors.c:497-498`), the
shared `M_Random` index (`s_sound.c:330`), the unpadded masked-midtexture read
(`r_draw.c:142`), and others. Entries judged
to cite no behaviour of the original at all are in the dropped list, not here.
