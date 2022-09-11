FILE(GLOB DOOM_LINUX_SOURCES *.c)
add_executable(bin-doom
  ${DOOM_LINUX_SOURCES}
)
target_compile_features(bin-doom PUBLIC c_std_99)
target_compile_options(bin-doom PUBLIC
  -m32
  -g
  -Wall
  -DNORMALUNIX
  -DLINUX
)
target_link_options(bin-doom PUBLIC
  -m32
)
target_include_directories(bin-doom PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(bin-doom PUBLIC
  Xext
  X11
  nsl
  m
)
FILE(COPY ${DOOM_LINUX_IWADS} DESTINATION ${CMAKE_CURRENT_BINARY_DIR})
