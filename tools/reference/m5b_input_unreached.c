/*
 * M5b input driver (contract DOOM-M5B-INPUT-v1): the functions of files the driver does not compile that the
 * input path never reaches. Each is declared on its STUB line and aborts if called. The parameter lists are not
 * the genuine ones: no header is included, so the definitions cannot clash with the genuine declarations, and a
 * function that is never called needs none. The list is the set of symbols the linker reports undefined without
 * this file; tools/reference/m5b_input_build.py requires every defined symbol to be declared and vice versa.
 */
#include <stdio.h>
#include <stdlib.h>

static void unreached (const char *n) { fprintf (stderr, "%s reached (not modelled by the input driver)\n", n); abort (); }

/* STUB A_BFGSpray [unreached] p_pspr.c:781: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BFGSpray (void) { unreached ("A_BFGSpray"); }
/* STUB A_BFGsound [unreached] p_pspr.c:818: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BFGsound (void) { unreached ("A_BFGsound"); }
/* STUB A_BabyMetal [unreached] p_enemy.c:1771: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BabyMetal (void) { unreached ("A_BabyMetal"); }
/* STUB A_BossDeath [unreached] p_enemy.c:1609: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BossDeath (void) { unreached ("A_BossDeath"); }
/* STUB A_BrainAwake [unreached] p_enemy.c:1813: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BrainAwake (void) { unreached ("A_BrainAwake"); }
/* STUB A_BrainDie [unreached] p_enemy.c:1896: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BrainDie (void) { unreached ("A_BrainDie"); }
/* STUB A_BrainExplode [unreached] p_enemy.c:1875: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BrainExplode (void) { unreached ("A_BrainExplode"); }
/* STUB A_BrainPain [unreached] p_enemy.c:1843: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BrainPain (void) { unreached ("A_BrainPain"); }
/* STUB A_BrainScream [unreached] p_enemy.c:1849: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BrainScream (void) { unreached ("A_BrainScream"); }
/* STUB A_BrainSpit [unreached] p_enemy.c:1901: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BrainSpit (void) { unreached ("A_BrainSpit"); }
/* STUB A_BruisAttack [unreached] p_enemy.c:979: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BruisAttack (void) { unreached ("A_BruisAttack"); }
/* STUB A_BspiAttack [unreached] p_enemy.c:898: state action referenced by the info.c state table; no state is entered without the simulation */
void A_BspiAttack (void) { unreached ("A_BspiAttack"); }
/* STUB A_CPosAttack [unreached] p_enemy.c:845: state action referenced by the info.c state table; no state is entered without the simulation */
void A_CPosAttack (void) { unreached ("A_CPosAttack"); }
/* STUB A_CPosRefire [unreached] p_enemy.c:865: state action referenced by the info.c state table; no state is entered without the simulation */
void A_CPosRefire (void) { unreached ("A_CPosRefire"); }
/* STUB A_Chase [unreached] p_enemy.c:672: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Chase (void) { unreached ("A_Chase"); }
/* STUB A_CheckReload [unreached] p_pspr.c:366: state action referenced by the info.c state table; no state is entered without the simulation */
void A_CheckReload (void) { unreached ("A_CheckReload"); }
/* STUB A_CloseShotgun2 [unreached] p_enemy.c:1799: state action referenced by the info.c state table; no state is entered without the simulation */
void A_CloseShotgun2 (void) { unreached ("A_CloseShotgun2"); }
/* STUB A_CyberAttack [unreached] p_enemy.c:969: state action referenced by the info.c state table; no state is entered without the simulation */
void A_CyberAttack (void) { unreached ("A_CyberAttack"); }
/* STUB A_Explode [unreached] p_enemy.c:1598: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Explode (void) { unreached ("A_Explode"); }
/* STUB A_FaceTarget [unreached] p_enemy.c:782: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FaceTarget (void) { unreached ("A_FaceTarget"); }
/* STUB A_Fall [unreached] p_enemy.c:1585: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Fall (void) { unreached ("A_Fall"); }
/* STUB A_FatAttack1 [unreached] p_enemy.c:1358: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FatAttack1 (void) { unreached ("A_FatAttack1"); }
/* STUB A_FatAttack2 [unreached] p_enemy.c:1375: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FatAttack2 (void) { unreached ("A_FatAttack2"); }
/* STUB A_FatAttack3 [unreached] p_enemy.c:1392: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FatAttack3 (void) { unreached ("A_FatAttack3"); }
/* STUB A_FatRaise [unreached] p_enemy.c:1351: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FatRaise (void) { unreached ("A_FatRaise"); }
/* STUB A_Fire [unreached] p_enemy.c:1257: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Fire (void) { unreached ("A_Fire"); }
/* STUB A_FireBFG [unreached] p_pspr.c:564: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FireBFG (void) { unreached ("A_FireBFG"); }
/* STUB A_FireCGun [unreached] p_pspr.c:733: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FireCGun (void) { unreached ("A_FireCGun"); }
/* STUB A_FireCrackle [unreached] p_enemy.c:1251: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FireCrackle (void) { unreached ("A_FireCrackle"); }
/* STUB A_FireMissile [unreached] p_pspr.c:551: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FireMissile (void) { unreached ("A_FireMissile"); }
/* STUB A_FirePistol [unreached] p_pspr.c:647: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FirePistol (void) { unreached ("A_FirePistol"); }
/* STUB A_FirePlasma [unreached] p_pspr.c:578: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FirePlasma (void) { unreached ("A_FirePlasma"); }
/* STUB A_FireShotgun [unreached] p_pspr.c:669: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FireShotgun (void) { unreached ("A_FireShotgun"); }
/* STUB A_FireShotgun2 [unreached] p_pspr.c:696: state action referenced by the info.c state table; no state is entered without the simulation */
void A_FireShotgun2 (void) { unreached ("A_FireShotgun2"); }
/* STUB A_GunFlash [unreached] p_pspr.c:449: state action referenced by the info.c state table; no state is entered without the simulation */
void A_GunFlash (void) { unreached ("A_GunFlash"); }
/* STUB A_HeadAttack [unreached] p_enemy.c:950: state action referenced by the info.c state table; no state is entered without the simulation */
void A_HeadAttack (void) { unreached ("A_HeadAttack"); }
/* STUB A_Hoof [unreached] p_enemy.c:1759: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Hoof (void) { unreached ("A_Hoof"); }
/* STUB A_KeenDie [unreached] p_enemy.c:566: state action referenced by the info.c state table; no state is entered without the simulation */
void A_KeenDie (void) { unreached ("A_KeenDie"); }
/* STUB A_Light0 [unreached] p_pspr.c:761: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Light0 (void) { unreached ("A_Light0"); }
/* STUB A_Light1 [unreached] p_pspr.c:766: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Light1 (void) { unreached ("A_Light1"); }
/* STUB A_Light2 [unreached] p_pspr.c:771: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Light2 (void) { unreached ("A_Light2"); }
/* STUB A_LoadShotgun2 [unreached] p_enemy.c:1786: state action referenced by the info.c state table; no state is entered without the simulation */
void A_LoadShotgun2 (void) { unreached ("A_LoadShotgun2"); }
/* STUB A_Look [unreached] p_enemy.c:604: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Look (void) { unreached ("A_Look"); }
/* STUB A_Lower [unreached] p_pspr.c:385: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Lower (void) { unreached ("A_Lower"); }
/* STUB A_Metal [unreached] p_enemy.c:1765: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Metal (void) { unreached ("A_Metal"); }
/* STUB A_OpenShotgun2 [unreached] p_enemy.c:1778: state action referenced by the info.c state table; no state is entered without the simulation */
void A_OpenShotgun2 (void) { unreached ("A_OpenShotgun2"); }
/* STUB A_Pain [unreached] p_enemy.c:1577: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Pain (void) { unreached ("A_Pain"); }
/* STUB A_PainAttack [unreached] p_enemy.c:1512: state action referenced by the info.c state table; no state is entered without the simulation */
void A_PainAttack (void) { unreached ("A_PainAttack"); }
/* STUB A_PainDie [unreached] p_enemy.c:1522: state action referenced by the info.c state table; no state is entered without the simulation */
void A_PainDie (void) { unreached ("A_PainDie"); }
/* STUB A_PlayerScream [unreached] p_enemy.c:1994: state action referenced by the info.c state table; no state is entered without the simulation */
void A_PlayerScream (void) { unreached ("A_PlayerScream"); }
/* STUB A_PosAttack [unreached] p_enemy.c:802: state action referenced by the info.c state table; no state is entered without the simulation */
void A_PosAttack (void) { unreached ("A_PosAttack"); }
/* STUB A_Punch [unreached] p_pspr.c:468: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Punch (void) { unreached ("A_Punch"); }
/* STUB A_Raise [unreached] p_pspr.c:423: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Raise (void) { unreached ("A_Raise"); }
/* STUB A_ReFire [unreached] p_enemy.c:1794: state action referenced by the info.c state table; no state is entered without the simulation */
void A_ReFire (void) { unreached ("A_ReFire"); }
/* STUB A_SPosAttack [unreached] p_enemy.c:821: state action referenced by the info.c state table; no state is entered without the simulation */
void A_SPosAttack (void) { unreached ("A_SPosAttack"); }
/* STUB A_SargAttack [unreached] p_enemy.c:935: state action referenced by the info.c state table; no state is entered without the simulation */
void A_SargAttack (void) { unreached ("A_SargAttack"); }
/* STUB A_Saw [unreached] p_pspr.c:502: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Saw (void) { unreached ("A_Saw"); }
/* STUB A_Scream [unreached] p_enemy.c:1535: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Scream (void) { unreached ("A_Scream"); }
/* STUB A_SkelFist [unreached] p_enemy.c:1101: state action referenced by the info.c state table; no state is entered without the simulation */
void A_SkelFist (void) { unreached ("A_SkelFist"); }
/* STUB A_SkelMissile [unreached] p_enemy.c:1002: state action referenced by the info.c state table; no state is entered without the simulation */
void A_SkelMissile (void) { unreached ("A_SkelMissile"); }
/* STUB A_SkelWhoosh [unreached] p_enemy.c:1093: state action referenced by the info.c state table; no state is entered without the simulation */
void A_SkelWhoosh (void) { unreached ("A_SkelWhoosh"); }
/* STUB A_SkullAttack [unreached] p_enemy.c:1419: state action referenced by the info.c state table; no state is entered without the simulation */
void A_SkullAttack (void) { unreached ("A_SkullAttack"); }
/* STUB A_SpawnFly [unreached] p_enemy.c:1936: state action referenced by the info.c state table; no state is entered without the simulation */
void A_SpawnFly (void) { unreached ("A_SpawnFly"); }
/* STUB A_SpawnSound [unreached] p_enemy.c:1930: state action referenced by the info.c state table; no state is entered without the simulation */
void A_SpawnSound (void) { unreached ("A_SpawnSound"); }
/* STUB A_SpidRefire [unreached] p_enemy.c:882: state action referenced by the info.c state table; no state is entered without the simulation */
void A_SpidRefire (void) { unreached ("A_SpidRefire"); }
/* STUB A_StartFire [unreached] p_enemy.c:1245: state action referenced by the info.c state table; no state is entered without the simulation */
void A_StartFire (void) { unreached ("A_StartFire"); }
/* STUB A_Tracer [unreached] p_enemy.c:1021: state action referenced by the info.c state table; no state is entered without the simulation */
void A_Tracer (void) { unreached ("A_Tracer"); }
/* STUB A_TroopAttack [unreached] p_enemy.c:913: state action referenced by the info.c state table; no state is entered without the simulation */
void A_TroopAttack (void) { unreached ("A_TroopAttack"); }
/* STUB A_VileAttack [unreached] p_enemy.c:1310: state action referenced by the info.c state table; no state is entered without the simulation */
void A_VileAttack (void) { unreached ("A_VileAttack"); }
/* STUB A_VileChase [unreached] p_enemy.c:1167: state action referenced by the info.c state table; no state is entered without the simulation */
void A_VileChase (void) { unreached ("A_VileChase"); }
/* STUB A_VileStart [unreached] p_enemy.c:1233: state action referenced by the info.c state table; no state is entered without the simulation */
void A_VileStart (void) { unreached ("A_VileStart"); }
/* STUB A_VileTarget [unreached] p_enemy.c:1285: state action referenced by the info.c state table; no state is entered without the simulation */
void A_VileTarget (void) { unreached ("A_VileTarget"); }
/* STUB A_WeaponReady [unreached] p_pspr.c:282: state action referenced by the info.c state table; no state is entered without the simulation */
void A_WeaponReady (void) { unreached ("A_WeaponReady"); }
/* STUB A_XScream [unreached] p_enemy.c:1572: state action referenced by the info.c state table; no state is entered without the simulation */
void A_XScream (void) { unreached ("A_XScream"); }
/* STUB D_CheckNetGame [unreached] d_net.c:555: start-up of D_DoomMain, which the driver replaces */
void D_CheckNetGame (void) { unreached ("D_CheckNetGame"); }
/* STUB F_Drawer [unreached] f_finale.c:700: drawing (D_Display, the drawers and the wipe); the driver never draws */
void F_Drawer (void) { unreached ("F_Drawer"); }
/* STUB F_Responder [unreached] f_finale.c:195: finale or intermission, reached only after a level is completed, which needs the simulation */
void F_Responder (void) { unreached ("F_Responder"); }
/* STUB F_StartFinale [unreached] f_finale.c:96: finale or intermission, reached only after a level is completed, which needs the simulation */
void F_StartFinale (void) { unreached ("F_StartFinale"); }
/* STUB F_Ticker [unreached] f_finale.c:207: finale or intermission, reached only after a level is completed, which needs the simulation */
void F_Ticker (void) { unreached ("F_Ticker"); }
/* STUB I_Init [unreached] i_system.c:107: start-up of D_DoomMain, which the driver replaces */
void I_Init (void) { unreached ("I_Init"); }
/* STUB I_SubmitSound [unreached] i_sound.c:666: sound update of D_DoomLoop, not run */
void I_SubmitSound (void) { unreached ("I_SubmitSound"); }
/* STUB NetUpdate [unreached] d_net.c:368: the network loop; the driver runs the singletics branch of D_DoomLoop itself (d_main.c:376-386) */
void NetUpdate (void) { unreached ("NetUpdate"); }
/* STUB P_ArchivePlayers [unreached] p_saveg.c:47: savegame code (G_DoSaveGame, G_DoLoadGame); no save or load runs */
void P_ArchivePlayers (void) { unreached ("P_ArchivePlayers"); }
/* STUB P_ArchiveSpecials [unreached] p_saveg.c:355: savegame code (G_DoSaveGame, G_DoLoadGame); no save or load runs */
void P_ArchiveSpecials (void) { unreached ("P_ArchiveSpecials"); }
/* STUB P_ArchiveThinkers [unreached] p_saveg.c:232: savegame code (G_DoSaveGame, G_DoLoadGame); no save or load runs */
void P_ArchiveThinkers (void) { unreached ("P_ArchiveThinkers"); }
/* STUB P_ArchiveWorld [unreached] p_saveg.c:114: savegame code (G_DoSaveGame, G_DoLoadGame); no save or load runs */
void P_ArchiveWorld (void) { unreached ("P_ArchiveWorld"); }
/* STUB P_CheckPosition [unreached] p_map.c:375: map and actor code of the map setup and the simulation, both replaced (P_SetupLevel and P_Ticker adapters) */
void P_CheckPosition (void) { unreached ("P_CheckPosition"); }
/* STUB P_Init [unreached] p_setup.c:700: start-up of D_DoomMain, which the driver replaces */
void P_Init (void) { unreached ("P_Init"); }
/* STUB P_RemoveMobj [unreached] p_mobj.c:546: map and actor code of the map setup and the simulation, both replaced (P_SetupLevel and P_Ticker adapters) */
void P_RemoveMobj (void) { unreached ("P_RemoveMobj"); }
/* STUB P_SpawnMobj [unreached] p_mobj.c:480: map and actor code of the map setup and the simulation, both replaced (P_SetupLevel and P_Ticker adapters) */
void P_SpawnMobj (void) { unreached ("P_SpawnMobj"); }
/* STUB P_SpawnPlayer [unreached] p_mobj.c:642: map and actor code of the map setup and the simulation, both replaced (P_SetupLevel and P_Ticker adapters) */
void P_SpawnPlayer (void) { unreached ("P_SpawnPlayer"); }
/* STUB P_UnArchivePlayers [unreached] p_saveg.c:79: savegame code (G_DoSaveGame, G_DoLoadGame); no save or load runs */
void P_UnArchivePlayers (void) { unreached ("P_UnArchivePlayers"); }
/* STUB P_UnArchiveSpecials [unreached] p_saveg.c:475: savegame code (G_DoSaveGame, G_DoLoadGame); no save or load runs */
void P_UnArchiveSpecials (void) { unreached ("P_UnArchiveSpecials"); }
/* STUB P_UnArchiveThinkers [unreached] p_saveg.c:266: savegame code (G_DoSaveGame, G_DoLoadGame); no save or load runs */
void P_UnArchiveThinkers (void) { unreached ("P_UnArchiveThinkers"); }
/* STUB P_UnArchiveWorld [unreached] p_saveg.c:167: savegame code (G_DoSaveGame, G_DoLoadGame); no save or load runs */
void P_UnArchiveWorld (void) { unreached ("P_UnArchiveWorld"); }
/* STUB R_DrawViewBorder [unreached] r_draw.c:843: drawing (D_Display, the drawers and the wipe); the driver never draws */
void R_DrawViewBorder (void) { unreached ("R_DrawViewBorder"); }
/* STUB R_ExecuteSetViewSize [unreached] r_main.c:671: drawing (D_Display, the drawers and the wipe); the driver never draws */
void R_ExecuteSetViewSize (void) { unreached ("R_ExecuteSetViewSize"); }
/* STUB R_FillBackScreen [unreached] r_draw.c:731: drawing (D_Display, the drawers and the wipe); the driver never draws */
void R_FillBackScreen (void) { unreached ("R_FillBackScreen"); }
/* STUB R_Init [unreached] r_main.c:773: start-up of D_DoomMain, which the driver replaces */
void R_Init (void) { unreached ("R_Init"); }
/* STUB R_PointInSubsector [unreached] r_main.c:801: map and actor code of the map setup and the simulation, both replaced (P_SetupLevel and P_Ticker adapters) */
void R_PointInSubsector (void) { unreached ("R_PointInSubsector"); }
/* STUB R_PointToAngle2 [unreached] r_main.c:378: map and actor code of the map setup and the simulation, both replaced (P_SetupLevel and P_Ticker adapters) */
void R_PointToAngle2 (void) { unreached ("R_PointToAngle2"); }
/* STUB R_RenderPlayerView [unreached] r_main.c:870: drawing (D_Display, the drawers and the wipe); the driver never draws */
void R_RenderPlayerView (void) { unreached ("R_RenderPlayerView"); }
/* STUB R_VideoErase [unreached] r_draw.c:818: drawing (D_Display, the drawers and the wipe); the driver never draws */
void R_VideoErase (void) { unreached ("R_VideoErase"); }
/* STUB S_Init [unreached] s_sound.c:161: start-up of D_DoomMain, which the driver replaces */
void S_Init (void) { unreached ("S_Init"); }
/* STUB S_UpdateSounds [unreached] s_sound.c:519: sound update of D_DoomLoop, not run */
void S_UpdateSounds (void) { unreached ("S_UpdateSounds"); }
/* STUB TryRunTics [unreached] d_net.c:636: the network loop; the driver runs the singletics branch of D_DoomLoop itself (d_main.c:376-386) */
void TryRunTics (void) { unreached ("TryRunTics"); }
/* STUB V_CopyRect [unreached] v_video.c:158: drawing (D_Display, the drawers and the wipe); the driver never draws */
void V_CopyRect (void) { unreached ("V_CopyRect"); }
/* STUB V_DrawPatch [unreached] f_finale.c:677: drawing (D_Display, the drawers and the wipe); the driver never draws */
void V_DrawPatch (void) { unreached ("V_DrawPatch"); }
/* STUB V_DrawPatchDirect [unreached] d_main.c:309: drawing (D_Display, the drawers and the wipe); the driver never draws */
void V_DrawPatchDirect (void) { unreached ("V_DrawPatchDirect"); }
/* STUB V_Init [unreached] v_video.c:482: start-up of D_DoomMain, which the driver replaces */
void V_Init (void) { unreached ("V_Init"); }
/* STUB V_MarkRect [unreached] am_map.c:298: drawing (D_Display, the drawers and the wipe); the driver never draws */
void V_MarkRect (void) { unreached ("V_MarkRect"); }
/* STUB WI_Drawer [unreached] wi_stuff.c:1772: drawing (D_Display, the drawers and the wipe); the driver never draws */
void WI_Drawer (void) { unreached ("WI_Drawer"); }
/* STUB WI_Start [unreached] wi_stuff.c:1838: finale or intermission, reached only after a level is completed, which needs the simulation */
void WI_Start (void) { unreached ("WI_Start"); }
/* STUB WI_Ticker [unreached] wi_stuff.c:1503: finale or intermission, reached only after a level is completed, which needs the simulation */
void WI_Ticker (void) { unreached ("WI_Ticker"); }
/* STUB W_InitMultipleFiles [unreached] w_wad.c:292: start-up of D_DoomMain, which the driver replaces */
void W_InitMultipleFiles (void) { unreached ("W_InitMultipleFiles"); }
/* STUB Z_Init [unreached] z_zone.c:93: start-up of D_DoomMain, which the driver replaces */
void Z_Init (void) { unreached ("Z_Init"); }
/* STUB wipe_EndScreen [unreached] f_wipe.c:249: drawing (D_Display, the drawers and the wipe); the driver never draws */
void wipe_EndScreen (void) { unreached ("wipe_EndScreen"); }
/* STUB wipe_ScreenWipe [unreached] f_wipe.c:262: drawing (D_Display, the drawers and the wipe); the driver never draws */
void wipe_ScreenWipe (void) { unreached ("wipe_ScreenWipe"); }
/* STUB wipe_StartScreen [unreached] f_wipe.c:237: drawing (D_Display, the drawers and the wipe); the driver never draws */
void wipe_StartScreen (void) { unreached ("wipe_StartScreen"); }
