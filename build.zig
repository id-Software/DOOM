const std = @import("std");

// Although this function looks imperative, note that its job is to
// declaratively construct a build graph that will be executed by an external
// runner.
pub fn build(b: *std.Build) void {
    // Standard target options allows the person running `zig build` to choose
    // what target to build for. Here we do not override the defaults, which
    // means any target is allowed, and the default is native. Other options
    // for restricting supported target set are available.
    const target = b.standardTargetOptions(.{});

    // Standard optimization options allow the person running `zig build` to select
    // between Debug, ReleaseSafe, ReleaseFast, and ReleaseSmall. Here we do not
    // set a preferred release mode, allowing the user to decide how to optimize.
    const optimize = b.standardOptimizeOption(.{});

    const exe = b.addExecutable(.{
        .name = "DOOM",
        // In this case the main source file is merely a path, however, in more
        // complicated build scripts, this could be a generated file.
        .root_source_file = .{ .path = "linuxdoom-1.10/i_main.c" },
        .target = target,
        .optimize = optimize,
    });

    const c_files = [_][]const u8{
        "linuxdoom-1.10/am_map.c",
        "linuxdoom-1.10/d_items.c",
        "linuxdoom-1.10/d_main.c",
        "linuxdoom-1.10/d_net.c",
        "linuxdoom-1.10/doomdef.c",
        "linuxdoom-1.10/doomstat.c",
        "linuxdoom-1.10/dstrings.c",
        "linuxdoom-1.10/f_finale.c",
        "linuxdoom-1.10/f_wipe.c",
        "linuxdoom-1.10/g_game.c",
        "linuxdoom-1.10/hu_lib.c",
        "linuxdoom-1.10/hu_stuff.c",
        "linuxdoom-1.10/i_net.c",
        // "linuxdoom-1.10/i_sound.c", DISABLE SOUND
        "linuxdoom-1.10/i_system.c",
        "linuxdoom-1.10/i_video.c",
        "linuxdoom-1.10/info.c",
        "linuxdoom-1.10/m_argv.c",
        "linuxdoom-1.10/m_bbox.c",
        "linuxdoom-1.10/m_cheat.c",
        "linuxdoom-1.10/m_fixed.c",
        "linuxdoom-1.10/m_menu.c",
        "linuxdoom-1.10/m_misc.c",
        "linuxdoom-1.10/m_random.c",
        "linuxdoom-1.10/m_swap.c",
        "linuxdoom-1.10/p_ceilng.c",
        "linuxdoom-1.10/p_doors.c",
        "linuxdoom-1.10/p_enemy.c",
        "linuxdoom-1.10/p_floor.c",
        "linuxdoom-1.10/p_inter.c",
        "linuxdoom-1.10/p_lights.c",
        "linuxdoom-1.10/p_map.c",
        "linuxdoom-1.10/p_maputl.c",
        "linuxdoom-1.10/p_mobj.c",
        "linuxdoom-1.10/p_plats.c",
        "linuxdoom-1.10/p_pspr.c",
        "linuxdoom-1.10/p_saveg.c",
        "linuxdoom-1.10/p_setup.c",
        "linuxdoom-1.10/p_sight.c",
        "linuxdoom-1.10/p_spec.c",
        "linuxdoom-1.10/p_switch.c",
        "linuxdoom-1.10/p_telept.c",
        "linuxdoom-1.10/p_tick.c",
        "linuxdoom-1.10/p_user.c",
        "linuxdoom-1.10/r_bsp.c",
        "linuxdoom-1.10/r_data.c",
        "linuxdoom-1.10/r_draw.c",
        "linuxdoom-1.10/r_main.c",
        "linuxdoom-1.10/r_plane.c",
        "linuxdoom-1.10/r_segs.c",
        "linuxdoom-1.10/r_sky.c",
        "linuxdoom-1.10/r_things.c",
        "linuxdoom-1.10/s_sound.c",
        "linuxdoom-1.10/sounds.c",
        "linuxdoom-1.10/st_lib.c",
        "linuxdoom-1.10/st_stuff.c",
        "linuxdoom-1.10/tables.c",
        "linuxdoom-1.10/v_video.c",
        "linuxdoom-1.10/w_wad.c",
        "linuxdoom-1.10/wi_stuff.c",
        "linuxdoom-1.10/z_zone.c",
    };

    const flags = [_][]const u8{ "-DLINUX", "-DNORMALUNIX" };
    for (c_files) |file| {
        exe.addCSourceFile(file, &flags);
    }

    exe.linkLibC();
    // We have defined our own little interface (the `ZwareDoom*` functions in i_video.c).
    // The host needs to provide these functions. However, by default, undefined symbols
    // will cause the wasm linker to error. We want to have the .wasm generate imports
    // for undefined symbols and we can do this with `exe.import_symbols = true` (equivalent
    // to wasm-ld `--allow-undefined`)
    exe.import_symbols = true;

    // This declares intent for the executable to be installed into the
    // standard location when the user invokes the "install" step (the default
    // step when running `zig build`).
    b.installArtifact(exe);

    // This *creates* a Run step in the build graph, to be executed when another
    // step is evaluated that depends on it. The next line below will establish
    // such a dependency.
    const run_cmd = b.addRunArtifact(exe);

    // By making the run step depend on the install step, it will be run from the
    // installation directory rather than directly from within the cache directory.
    // This is not necessary, however, if the application depends on other installed
    // files, this ensures they will be present and in the expected location.
    run_cmd.step.dependOn(b.getInstallStep());

    // This allows the user to pass arguments to the application in the build
    // command itself, like this: `zig build run -- arg1 arg2 etc`
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    // This creates a build step. It will be visible in the `zig build --help` menu,
    // and can be selected like this: `zig build run`
    // This will evaluate the `run` step rather than the default, which is "install".
    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);

    // Creates a step for unit testing. This only builds the test executable
    // but does not run it.
    const unit_tests = b.addTest(.{
        .root_source_file = .{ .path = "src/main.zig" },
        .target = target,
        .optimize = optimize,
    });

    const run_unit_tests = b.addRunArtifact(unit_tests);

    // Similar to creating the run step earlier, this exposes a `test` step to
    // the `zig build --help` menu, providing a way for the user to request
    // running the unit tests.
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_unit_tests.step);
}
