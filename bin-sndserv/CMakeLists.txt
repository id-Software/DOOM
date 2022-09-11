FILE(GLOB DOOM_SNDSERV_LINUX_SOURCES *.c)
add_executable(bin-sndserv
  ${DOOM_SNDSERV_LINUX_SOURCES}
)
target_compile_features(bin-sndserv PUBLIC c_std_99)
target_compile_options(bin-sndserv PUBLIC
  -m32
  -g
  -Wall
  -DNORMALUNIX
  -DLINUX
)
target_link_options(bin-sndserv PUBLIC
  -m32
)
target_include_directories(bin-sndserv PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(bin-sndserv PUBLIC
  m
)
FILE(COPY ${DOOM_LINUX_IWADS} DESTINATION ${CMAKE_CURRENT_BINARY_DIR})
