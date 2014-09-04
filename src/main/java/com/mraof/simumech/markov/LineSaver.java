package com.mraof.simumech.markov;

import com.mraof.simumech.Main;

public class LineSaver implements Runnable 
{
	public MarkovChain markovChain;

	public LineSaver(MarkovChain markovChain) 
	{
		this.markovChain = markovChain;
	}
	
	@Override
	public void run() 
	{
		do
		{
			try {
				Thread.sleep(60000);
			} catch (InterruptedException e) {System.out.println("Autosave forced to wake up, saving");}
			this.markovChain.save();
//			System.out.println("Saved");
		} while(Main.running);
		System.out.println("Line saver stopped");
	}

}
