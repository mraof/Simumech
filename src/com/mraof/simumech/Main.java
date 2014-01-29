package com.mraof.simumech;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.util.HashMap;

import com.mraof.simumech.markov.LineSaver;
import com.mraof.simumech.markov.MarkovChain;

public class Main 
{
	public static String clientName = "Simumech";
	public static String version = "0";

	public static String[] owners = {"Mraof"};
	public static MarkovChain markovChain;
	public static boolean useCR = true;
	public static boolean running = true;
	public static HashMap<String, IChat> chats = new HashMap<String, IChat>();

	public static void main(String args[])
	{
		if(args.length >= 1 && args[0].equals("noCR"));
		useCR = false;
		markovChain = new MarkovChain();
		LineSaver lineSaver = new LineSaver(markovChain);
		Thread autosave = new Thread(lineSaver);
		autosave.start();
		

		ChatLoader chatLoader = new ChatLoader();


		try
		{
			chats.put("irc", (IChat) (Class.forName("com.mraof.simumech.irc.IRC", false, chatLoader)).newInstance());
		}catch(InstantiationException | IllegalAccessException | ClassNotFoundException e){e.printStackTrace();};
		try
		{
			chats.put("skype", (IChat) (Class.forName("com.mraof.simumech.skype.SkypeBot", false, chatLoader)).newInstance());
		}catch(InstantiationException | IllegalAccessException | ClassNotFoundException e){e.printStackTrace();};

		BufferedReader bufferedReader = new BufferedReader(new InputStreamReader(System.in));
		String inputString;
		if(Profiler.instrumentation != null)
		{
			System.out.println("markovChain: " + Profiler.deepSizeOf(markovChain));
			System.out.println("chats: " + Profiler.deepSizeOf(chats));
		}

		try {
			while((inputString = bufferedReader.readLine()) != null)
			{
				if(inputString.equalsIgnoreCase("QUIT"))
				{
					break;
				}
				int splitIndex = inputString.indexOf(' ');
				if(splitIndex != -1)
				{
					String firstWord = inputString.substring(0, splitIndex).toLowerCase();
					inputString = inputString.substring(splitIndex + 1);
					if(firstWord.equals("reload"))
					{
						IChat chat = chats.get(inputString);					
						if(chat != null)
						{
							chat.quit();
							System.out.println("Reloading " + inputString + " (" +chat.getClass().getName() + ")");
							try
							{
								chats.put(firstWord, (IChat) (Class.forName(chat.getClass().getName(), false, chatLoader)).newInstance());
							} catch (InstantiationException e) {
								e.printStackTrace();
							} catch (IllegalAccessException e) {
								e.printStackTrace();
							} catch (ClassNotFoundException e) {
								e.printStackTrace();
							}
							System.out.println("Done");
						}
					}
				}
				//System.out.println(markovChain.reply(inputString));
			}
		} catch (IOException e) {
			e.printStackTrace();
		}
		running = false;
		System.out.println("Input loop done");

		for(IChat chat : chats.values())
			chat.quit();
		autosave.interrupt();

		try {
			Thread.sleep(5000);
		} catch (InterruptedException e) {e.printStackTrace();}


	}
}
