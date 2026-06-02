module DSP_TEST_TOP (in0, in1, in2, in3, in4);
  input in0, in1, in2, in3, in4;

  // internal nets/constants
  wire clk_i;
  wire rst_ni;
  wire [31:0] FUNCT_SELECT_MASK;
  wire [1:0] MVTYPE_DSP;
  wire [0:0] recover_state_wires;
  wire [0:0] halt_dsp_lat;
  wire [0:0] mul_stage_1_en;
  wire [0:0] mul_stage_2_en;
  wire [0:0] mul_en;
  wire [63:0] dsp_in_mul_operands;
  wire [31:0] dsp_out_mul_results;

  // constant assignments per Multiplier IO description
  assign clk_i = 1'b0;
  assign rst_ni = 1'b1;
  assign FUNCT_SELECT_MASK = 32'hFFFFFFFF;
  assign MVTYPE_DSP = 2'b10;
  assign recover_state_wires = 1'b0;
  assign halt_dsp_lat = 1'b0;
  assign mul_stage_1_en = 1'b1;
  assign mul_stage_2_en = 1'b1;
  assign mul_en = 1'b1;

  // build the 64-bit input vector using the 5 scalar inputs in the LSBs
  assign dsp_in_mul_operands = {59'b0, in4, in3, in2, in1, in0};

  // Instantiate the MULTIPLIER (module implementation is in the original file)
  MULTIPLIER_multithreaded_accl_en0_SIMD1_ACCL_NUM1_FU_NUM1_Data_Width32_SIMD_Width32 mul_inst (
    .clk_i(clk_i),
    .rst_ni(rst_ni),
    .FUNCT_SELECT_MASK(FUNCT_SELECT_MASK),
    .MVTYPE_DSP(MVTYPE_DSP),
    .recover_state_wires(recover_state_wires),
    .halt_dsp_lat(halt_dsp_lat),
    .mul_stage_1_en(mul_stage_1_en),
    .mul_stage_2_en(mul_stage_2_en),
    .mul_en(mul_en),
    .dsp_in_mul_operands(dsp_in_mul_operands),
    .dsp_out_mul_results(dsp_out_mul_results)
  );

endmodule
